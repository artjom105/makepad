
use makepad_render::*;
use std::collections::{HashMap, BTreeMap};

#[derive(Clone, Eq, Copy, Ord, PartialOrd, PartialEq, Hash, Default)]
pub struct XRUserId(pub u32);

#[derive(Clone, Default)]
pub struct XRChannel {
    pub self_id: XRUserId,
    pub users: HashMap<XRUserId, XRUpdateEvent>,
    pub last_times: HashMap<XRUserId, f64>
}

#[derive(Clone)]
pub struct XRCube {
    pub cube: Cube,
    pub _area: Area,
    pub _mat: Mat4,
}

#[derive(Clone)]
pub struct XRCursor {
    pub quad: Quad,
    pub cursor_size: f32,
    pub _area: Area,
    pub _pt: Vec2,
}

impl XRCube {
    pub fn new(cx: &mut Cx) -> Self {
        Self {
            cube: Cube::new(cx),
            _area: Area::Empty,
            _mat: Mat4::identity()
        }
    }
    
    pub fn set_mat(&mut self, cx: &mut Cx, mat: Mat4) {
        self._mat = mat;
        self._area.write_mat4(cx, live_item_id!(makepad_render::cube::shader::transform), &mat);
    }
    
    pub fn draw_cube(&mut self, cx: &mut Cx, size: Vec3, pos: Vec3) {
        self._area = self.cube.draw_cube(cx, size, pos, &self._mat).into();
    }
}

impl XRCursor {
    fn new(cx: &mut Cx) -> Self {
        Self {
            quad: Quad {
                z: 3.0,
                ..Quad::new(cx)
            },
            cursor_size: 10.,
            _area: Area::Empty,
            _pt: Vec2::default()
        }
    }
    
    fn set_pt(&mut self, cx: &mut Cx, pt: Vec2) {
        self._pt = pt;
        self._area.write_float(cx, live_item_id!(makepad_render::quad::shader::x), pt.x - 0.5 * self.cursor_size);
        self._area.write_float(cx, live_item_id!(makepad_render::quad::shader::y), pt.y - 0.5 * self.cursor_size);
    }
    
    fn draw_cursor(&mut self, cx: &mut Cx) {
        self._area = self.quad.draw_quad_rel(cx, Rect {
            x: self._pt.x - 0.5 * self.cursor_size,
            y: self._pt.y - 0.5 * self.cursor_size,
            w: self.cursor_size,
            h: self.cursor_size
        }).into();
    }
}

#[derive(Clone)]
pub enum XRAvatarState {
    Joining(f32),
    Leaving(f32),
    Gone,
    Present
}

impl XRAvatarState {
    fn gone(&self) -> bool {
        match self {
            Self::Gone => true,
            _ => false
        }
    }
    
    fn get_space(&self) -> f32 {
        match self {
            Self::Joining(v) => 1.0 - *v,
            Self::Leaving(v) => *v,
            Self::Gone => 0.0,
            Self::Present => 1.0
        }
    }
    
    fn tick(&mut self) {
        match self {
            Self::Joining(f) => {
                if *f < 0.001 {
                    *self = Self::Present;
                }
                else {
                    *self = Self::Joining(*f * 0.99);
                }
            }
            Self::Leaving(f) => {
                if *f < 0.001 {
                    *self = Self::Gone;
                }
                else {
                    *self = Self::Leaving(*f * 0.9);
                }
            },
            _ => ()
        }
    }
    
    fn leave(&mut self) {
        match self {
            Self::Present => {
                *self = Self::Leaving(1.0);
            }
            Self::Joining(f) => {
                *self = Self::Leaving(1.0 - *f);
            }
            _ => ()
        }
    }
    
    fn join(&mut self) {
        match self {
            Self::Gone => {
                *self = Self::Joining(1.0);
            }
            Self::Leaving(f) => {
                *self = Self::Joining(1.0 - *f);
            }
            _ => ()
        }
    }
}

#[derive(Clone)]
pub struct XRAvatar {
    state: XRAvatarState,
    left_hand: XRCube,
    right_hand: XRCube,
    head: XRCube,
    angle: f32,
    ui: XRCube,
    ui_rect: Rect,
    last_xr_event: Option<XRUpdateEvent>
}

impl XRAvatar {
    fn new(cx: &mut Cx) -> Self {
        Self {
            state: XRAvatarState::Joining(1.0),
            left_hand: XRCube::new(cx),
            right_hand: XRCube::new(cx),
            last_xr_event: None,
            angle: 180.0,
            head: XRCube::new(cx),
            ui: XRCube::new(cx),
            ui_rect: Rect::default()
        }
    }
    
    fn update_avatar(&mut self, cx: &mut Cx, xr_event: Option<&XRUpdateEvent>, ui_mat: Mat4, ui_rect: Rect) {
        
        let personal_mat = Mat4::from_mul(
            &Mat4::scale_translate(self.state.get_space(), 0.0, 0.0, 0.0),
            &Mat4::rotate_tsrt(
                Vec3 {x: 0.0, y: 0.0, z: 1.5},
                1.0,
                Vec3 {x: 0.0, y: self.angle, z: 0.0},
                Vec3 {x: 0.0, y: 0.0, z: -1.5},
            )
        );
        let xr_event = if let Some(xe) = xr_event {
            self.last_xr_event = Some(xe.clone());
            xe
        } else {
            if let Some(xe) = &self.last_xr_event {xe} else {return}
        };
        
        self.left_hand.set_mat(cx, Mat4::from_mul(&Mat4::from_transform(xr_event.left_input.ray), &personal_mat));
        self.right_hand.set_mat(cx, Mat4::from_mul(&Mat4::from_transform(xr_event.right_input.ray), &personal_mat));
        self.head.set_mat(cx, Mat4::from_mul(&Mat4::from_transform(xr_event.head_transform), &personal_mat));
        self.ui.set_mat(cx, Mat4::from_mul(&ui_mat, &personal_mat));
        self.ui_rect = ui_rect;
    }
    
    fn draw_avatar(&mut self, cx: &mut Cx) {
        self.left_hand.cube.shader = live_shader!(cx, self::shader_hand);
        self.right_hand.cube.shader = live_shader!(cx, self::shader_hand);
        self.head.cube.shader = live_shader!(cx, self::shader_hand);
        let hand_size = Vec3 {x: 0.02, y: 0.02, z: 0.12};
        let hand_pos = Vec3 {x: 0., y: 0., z: 0.0};
        
        self.left_hand.draw_cube(cx, hand_size, hand_pos);
        self.right_hand.draw_cube(cx, hand_size, hand_pos);
        
        let head_pos = Vec3 {x: 0., y: 0., z: 0.0};
        let head_size = Vec3 {x: 0.20, y: 0.08, z: 0.10};
        self.head.draw_cube(cx, head_size, head_pos);
        
        let ui_pos = Vec3 {x: self.ui_rect.x + 0.5 * self.ui_rect.w, y: self.ui_rect.y + 0.5 * self.ui_rect.h, z: 0.};
        let ui_size = Vec3 {x: self.ui_rect.w, y: self.ui_rect.h, z: 25.0};
        self.ui.draw_cube(cx, ui_size, ui_pos);
    }
}

#[derive(Clone)]
pub struct XRControl {
    pub cursor_view: View,
    pub space_view: View,
    pub last_xr_update: Option<XRUpdateEvent>,
    
    pub xr_avatars: BTreeMap<XRUserId, XRAvatar>,

    pub left_hand: XRCube,
    pub right_hand: XRCube,
    pub left_cursor: XRCursor,
    pub right_cursor: XRCursor,
}

pub enum XRControlEvent {
    None
}

impl XRControl {
    pub fn new(cx: &mut Cx) -> Self {
        Self {
            space_view: View::new(cx),
            cursor_view: View::new(cx),
            last_xr_update: None,
            left_hand: XRCube::new(cx),
            right_hand: XRCube::new(cx),
            left_cursor: XRCursor::new(cx),
            right_cursor: XRCursor::new(cx),
            xr_avatars: BTreeMap::new(),
        }
    }
    
    pub fn style(cx: &mut Cx) {
        // lets define the shader
        live_body!(cx, r#"
            self::shader_hand: Shader {
                use makepad_render::cube::shader::*;
            }
            
            self::shader_cursor: Shader {
                use makepad_render::quad::shader::*;
                fn pixel() -> vec4 {
                    let df = Df::viewport(pos * vec2(w, h));
                    df.circle(0.5 * w, 0.5 * h, 0.5 * w);
                    return df.fill(#f);
                }
            }
        "#)
    }
    
    pub fn process_avatar_state(&mut self, cx: &mut Cx, xr_channel: &XRChannel, ui_mat: Mat4, ui_rect: Rect) {
        
        // compute ordered circle
        let mut circle = Vec::new();
        let mut insert = false;
        for (id, _) in &mut self.xr_avatars {
            if *id == xr_channel.self_id {
                insert = true
            }
            else if insert {
                circle.push(*id);
            }
        }
        for (id, _) in &mut self.xr_avatars {
            if *id == xr_channel.self_id {
                break
            }
            circle.push(*id);
        }
        
        // compute space circle takes up
        let mut total_space = 1.0;
        for id in &circle {
            if let Some(xa) = self.xr_avatars.get(id) {
                total_space += xa.state.get_space();
            }
        }
        
        let mut angle = 0.0;
        for id in &circle {
            if let Some(xa) = self.xr_avatars.get_mut(id) {
                angle += (360.0 / total_space) * xa.state.get_space();
                xa.angle = angle;
            }
        }
        
        // ok lets update the states
        for (id, _xe) in &xr_channel.users {
            if let Some(xa) = self.xr_avatars.get_mut(id) {
                xa.state.join();
            }
            else {
                self.xr_avatars.insert(*id, XRAvatar::new(cx));
                self.space_view.redraw_view_area(cx);
            }
        }
        
        let mut remove = Vec::new();
        for (id, xa) in &mut self.xr_avatars {
            
            xa.update_avatar(cx, xr_channel.users.get(id), ui_mat, ui_rect);
            xa.state.tick();
            
            if xr_channel.users.get(id).is_none() {
                xa.state.leave();
            }
            if xa.state.gone() {
                remove.push(*id);
                self.space_view.redraw_view_area(cx);
            }
        }
        for id in remove {
            self.xr_avatars.remove(&id);
            self.space_view.redraw_view_area(cx);
        }
    }
    
    pub fn handle_xr_control(
        &mut self,
        cx: &mut Cx,
        xr_event: &XRUpdateEvent,
        xr_channel: &XRChannel,
        window_view: &View
    ) -> Vec<Event> {
        
        // lets send our avatar over the socket
        let view_rect = window_view.get_rect(cx);
        
        let window_mat = Mat4::rotate_tsrt(
            Vec3 {x: 0., y: -view_rect.h, z: 0.0},
            -0.0005,
            Vec3 {x: 50.0, y: -180.0, z: 0.0},
            Vec3 {x: -0.20, y: -0.45, z: -0.3},
        );
        
        let inv_window_mat = window_mat.invert();
        window_view.set_view_transform(cx, &window_mat);
        
        // lets make a test with just us.
        self.process_avatar_state(cx, xr_channel, window_mat, view_rect);
        
        // lets set the left_input matrix
        self.left_hand.set_mat(cx, Mat4::from_transform(xr_event.left_input.ray));
        self.right_hand.set_mat(cx, Mat4::from_transform(xr_event.right_input.ray));
        self.last_xr_update = Some(xr_event.clone());
        
        fn get_intersect_pt(window_plane: &Plane, inv_window_mat: &Mat4, ray_mat: &Mat4) -> Vec2 {
            let origin = inv_window_mat.transform_vec4(ray_mat.transform_vec4(Vec4 {x: 0., y: 0., z: 0., w: 1.0}));
            let vector = inv_window_mat.transform_vec4(ray_mat.transform_vec4(Vec4 {x: 0., y: 0., z: 1., w: 1.0}));
            window_plane.intersect_line(origin.to_vec3(), vector.to_vec3()).to_vec2()
        }
        // we now simply need to intersect with the plane view_rect.w, view_rect.h, 0.
        let window_plane = Plane::from_points(
            Vec3 {x: 0., y: 0., z: 0.},
            Vec3 {x: view_rect.w, y: 0., z: 0.},
            Vec3 {x: 0., y: view_rect.h, z: 0.}
        );
        
        self.left_cursor.set_pt(cx, get_intersect_pt(&window_plane, &inv_window_mat, &self.left_hand._mat));
        self.right_cursor.set_pt(cx, get_intersect_pt(&window_plane, &inv_window_mat, &self.right_hand._mat));
        
        let mut events = Vec::new();
        
        fn do_input_event(events: &mut Vec<Event>, cx: &mut Cx, digit: usize, pt: Vec2, time: f64, input: &XRInput, last_input: &XRInput) {
            fn axis_not_zero(axis: f32) -> bool {
                axis < -0.01 || axis>0.01
            }
            if axis_not_zero(input.axes[2]) || axis_not_zero(input.axes[3]) {
                events.push(Event::FingerScroll(FingerScrollEvent {
                    window_id: 0,
                    digit: digit,
                    abs: pt,
                    rel: pt,
                    rect: Rect::default(),
                    handled_x: false,
                    handled_y: false,
                    scroll: Vec2 {x: input.axes[2] * 15.0, y: input.axes[3] * 15.0},
                    is_wheel: true,
                    modifiers: KeyModifiers::default(),
                    time: time
                }));
            }
            
            if input.buttons[0].pressed != last_input.buttons[0].pressed {
                log!("INPUT BUTTON PRESSED");
                // we have finger up or down
                if input.buttons[0].pressed {
                    events.push(Event::FingerDown(FingerDownEvent {
                        digit: digit,
                        window_id: 0,
                        tap_count: 0,
                        abs: pt,
                        rel: pt,
                        handled: false,
                        is_touch: true,
                        rect: Rect::default(),
                        modifiers: KeyModifiers::default(),
                        time: time
                    }));
                }
                else {
                    events.push(Event::FingerUp(FingerUpEvent {
                        digit: digit,
                        window_id: 0,
                        abs: pt,
                        rel: pt,
                        is_over: false,
                        is_touch: true,
                        rect: Rect::default(),
                        abs_start: Vec2::default(),
                        rel_start: Vec2::default(),
                        modifiers: KeyModifiers::default(),
                        time: time
                    }));
                }
                
            }
            else if input.buttons[0].pressed { // we have move
                events.push(Event::FingerMove(FingerMoveEvent {
                    digit: digit,
                    window_id: 0,
                    abs: pt,
                    rel: pt,
                    rect: Rect::default(),
                    abs_start: Vec2::default(),
                    rel_start: Vec2::default(),
                    is_over: false,
                    is_touch: true,
                    modifiers: KeyModifiers::default(),
                    time: time
                }));
            }
            else {
                cx.fingers[digit].over_last = Area::Empty;
                events.push(Event::FingerHover(FingerHoverEvent {
                    digit: digit,
                    any_down: false,
                    window_id: 0,
                    abs: pt,
                    rel: pt,
                    rect: Rect::default(),
                    handled: false,
                    hover_state: HoverState::Over,
                    modifiers: KeyModifiers::default(),
                    time: time
                }));
            }
        }
        do_input_event(&mut events, cx, 0, self.left_cursor._pt, xr_event.time, &xr_event.left_input, &xr_event.last_left_input);
        do_input_event(&mut events, cx, 1, self.right_cursor._pt, xr_event.time, &xr_event.right_input, &xr_event.last_right_input);
        events
    }
    
    pub fn draw_xr_control(&mut self, cx: &mut Cx) {
        self.left_hand.cube.shader = live_shader!(cx, self::shader_hand);
        self.right_hand.cube.shader = live_shader!(cx, self::shader_hand);
        self.left_cursor.quad.shader = live_shader!(cx, self::shader_cursor);
        self.right_cursor.quad.shader = live_shader!(cx, self::shader_cursor);
        
        // THIS HAS A VERY STRANGE BUG. if i reverse these, the dots are broken on wasm+quest
        if self.space_view.begin_view(cx, Layout::abs_origin_zero()).is_ok() {
            self.space_view.block_set_view_transform(cx);
            let hand_size = Vec3 {x: 0.02, y: 0.02, z: 0.12};
            let hand_pos = Vec3 {x: 0., y: 0., z: 0.0};
            
            self.left_hand.draw_cube(cx, hand_size, hand_pos);
            self.right_hand.draw_cube(cx, hand_size, hand_pos);
            
            for (_id, avatar) in &mut self.xr_avatars {
                avatar.draw_avatar(cx);
            }
            
            self.space_view.end_view(cx);
        }
        
        if self.cursor_view.begin_view(cx, Layout::abs_origin_zero()).is_ok() {
            self.left_cursor.draw_cursor(cx);
            self.right_cursor.draw_cursor(cx);
            self.cursor_view.end_view(cx);
        }
    }
}
