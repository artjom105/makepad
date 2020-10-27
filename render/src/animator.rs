use crate::cx::*;

#[derive(Clone)]
pub struct AnimInfo {
    pub start_time: f64,
    pub total_time: f64
}

#[derive(Clone)]
pub enum AnimLastValue {
    Float(f32), 
    Vec2(Vec2), 
    Vec3(Vec3),
    Vec4(Vec4),
    Color(Color),
}

#[derive(Default, Clone)]
pub struct Animator {
    current: Option<Anim>,
    next: Option<Anim>,
    pub area: Area,
    pub live_update_id: u64,
    pub last_values: Vec<(LiveItemId, AnimLastValue)>,
}

impl Animator {

    pub fn init<F>(&mut self, cx: &mut Cx, cb: F)
    where F: Fn(&Cx) -> Anim {
        if self.live_update_id != cx.live_update_id {
            self.live_update_id = cx.live_update_id;
            let anim = cb(cx);
            // lets stop all animations if we had any
            if let Some(anim_area) = cx.playing_anim_areas.get_mut(&self.area) {
                anim_area.total_time = 0.;
            }
            self.set_anim_as_last_values(&anim);
        }
    }
    
    pub fn set_anim_as_last_values(&mut self, anim: &Anim) {
        for track in &anim.tracks {
            // we dont have a last float, find it in the tracks
            let bind_id = track.bind_id();
            match track {
                Track::Color{keys,..} => {
                    let val = if keys.len()>0 {keys.last().unwrap().1}else {Color::default()};
                    if let Some((_name, value)) = self.last_values.iter_mut().find( | (name, _) | *name == bind_id) {
                        *value = AnimLastValue::Color(val);
                    }
                    else {
                        self.last_values.push((bind_id, AnimLastValue::Color(val)));
                    }
                },
                Track::Vec4{keys,..} => {
                    let val = if keys.len()>0 {keys.last().unwrap().1}else {Vec4::default()};
                    if let Some((_name, value)) = self.last_values.iter_mut().find( | (name, _) | *name == bind_id) {
                        *value = AnimLastValue::Vec4(val);
                    }
                    else {
                        self.last_values.push((bind_id, AnimLastValue::Vec4(val)));
                    }
                },
                Track::Vec3{keys,..} => {
                    let val = if keys.len()>0 {keys.last().unwrap().1}else {Vec3::default()};
                    if let Some((_name, value)) = self.last_values.iter_mut().find( | (name, _) | *name == bind_id) {
                        *value = AnimLastValue::Vec3(val);
                    }
                    else {
                        self.last_values.push((bind_id, AnimLastValue::Vec3(val)));
                    }
                },
                Track::Vec2{keys,..} => {
                    let val = if keys.len()>0 {keys.last().unwrap().1}else {Vec2::default()};
                    if let Some((_name, value)) = self.last_values.iter_mut().find( | (name, _) | *name == bind_id) {
                        *value = AnimLastValue::Vec2(val);
                    }
                    else {
                        self.last_values.push((bind_id, AnimLastValue::Vec2(val)));
                    }
                },
                Track::Float{keys,..} => {
                    let val = if keys.len()>0 {keys.last().unwrap().1}else {0.};
                    if let Some((_name, value)) = self.last_values.iter_mut().find( | (name, _) | *name == bind_id) {
                        *value = AnimLastValue::Float(val); 
                    }
                    else {
                        self.last_values.push((bind_id, AnimLastValue::Float(val)));
                    }
                },
            }
        }
    }
    
    pub fn end(&mut self) {
        if let Some(current) = self.current.take() {
            self.set_anim_as_last_values(&current);
        }
    }
    
    pub fn end_and_set(&mut self, anim: Anim) {
        self.current = None;
        self.set_anim_as_last_values(&anim);
    }
    
    pub fn term_anim_playing(&mut self) -> bool {
        if let Some(current) = &self.current {
            return current.play.term();
        }
        return false
    }
    
    pub fn play_anim(&mut self, cx: &mut Cx, anim: Anim) {
        self.live_update_id = cx.live_update_id;
        // if our area is invalid, we should just set our default value
        if let Some(current) = &self.current {
            if current.play.term() { // can't override a term anim
                return
            }
        }

        if !self.area.is_valid(cx) {
            self.set_anim_as_last_values(&anim);
            self.current = Some(anim);
            return
        }
        // alright first we find area, it already exists
        if let Some(anim_info) = cx.playing_anim_areas.get_mut(&self.area){
            //do we cut the animation in right now?
            if anim.play.cut() || self.current.is_none() {
                self.current = Some(anim);
                anim_info.start_time = std::f64::NAN;
                self.next = None;
                anim_info.total_time = self.current.as_ref().unwrap().play.total_time();
            }
            else { // queue it
                self.next = Some(anim);
                // lets ask an animation anim how long it is
                anim_info.total_time = self.current.as_ref().unwrap().play.total_time() + self.next.as_ref().unwrap().play.total_time()
            }
        }
        else if self.area != Area::Empty { // its new
            self.current = Some(anim);
            self.next = None;
            cx.playing_anim_areas.insert(self.area, AnimInfo {
                start_time: std::f64::NAN,
                total_time: self.current.as_ref().unwrap().play.total_time()
            });
        }
    }
    
    pub fn set_area(&mut self, cx: &mut Cx, area: Area) {
        self.area = cx.update_area_refs(self.area, area.clone());
    }
    
    
    pub fn update_anim_track(&mut self, cx: &mut Cx, time: f64) -> Option<f64> {

        // alright first we find area in running animations

        // fetch current anim
        if self.current.is_none() { // remove anim
            cx.playing_anim_areas.remove(&self.area);
            return None
        }
        
        if let Some(anim_info) = cx.playing_anim_areas.get_mut(&self.area){
            if anim_info.start_time.is_nan(){
                anim_info.start_time = time;
            }
            
            let current_total_time = self.current.as_ref().unwrap().play.total_time();
        
            // process queueing
            if time - anim_info.start_time >= current_total_time && !self.next.is_none() {
                self.current = self.next.clone();
                self.next = None;
                // update animation slot
                anim_info.start_time += current_total_time;
                anim_info.total_time -= current_total_time;

                Some(self.current.as_ref().unwrap().play.compute_time(time - anim_info.start_time))
            }
            else {
                Some(self.current.as_ref().unwrap().play.compute_time(time - anim_info.start_time))
            }
            
        }
        else{
            return None
        }
    }
    
    pub fn find_track_index(&mut self, bind_id: LiveItemId) -> Option<usize> {
        // find our track
        for (track_index, track) in &mut self.current.as_ref().unwrap().tracks.iter().enumerate() {
            if track.bind_id() == bind_id {
                return Some(track_index);
            }
        }
        None
    }
    
    pub fn calc_float(&mut self, cx: &mut Cx, bind_id: LiveItemId, time: f64) -> f32 {
        let last = Self::_last_float(bind_id, &self.last_values);
        let mut ret = last;
        if let Some(time) = self.update_anim_track(cx, time) {
            if let Some(track_index) = self.find_track_index(bind_id) {
                if let Track::Float{keys, cut_init, ease, ..} = &mut self.current.as_mut().unwrap().tracks[track_index] {
                    ret = Track::compute_track_float(time, keys, cut_init, last, ease);
                }
            }
        }
        self.set_last_float(bind_id, ret);
        return ret
    }
    
    pub fn last_float(&self, _cx: &Cx, bind_id: LiveItemId) -> f32 {
        Self::_last_float(bind_id, &self.last_values)
    }
    
    pub fn _last_float(bind_id: LiveItemId, last_float: &Vec<(LiveItemId, AnimLastValue)>) -> f32 {
        if let Some((_, value)) = last_float.iter().find( | v | v.0 == bind_id) {
            if let AnimLastValue::Float(value) = value {
                return *value
            }
        }
        return 0.0
    }
    
    pub fn set_last_float(&mut self, bind_id: LiveItemId, value: f32) {
        Self::_set_last_float(bind_id, value, &mut self.last_values)
    }
    
    pub fn _set_last_float(bind_id: LiveItemId, value: f32, last_values: &mut Vec<(LiveItemId, AnimLastValue)>) {
        if let Some((_, last)) = last_values.iter_mut().find( | v | v.0 == bind_id) {
            *last = AnimLastValue::Float(value);
        }
        else {
            last_values.push((bind_id, AnimLastValue::Float(value)))
        }
    }
    
    pub fn calc_vec2(&mut self, cx: &mut Cx, bind_id: LiveItemId, time: f64) -> Vec2 {
        let last = Self::_last_vec2(bind_id, &self.last_values);
        let mut ret = last;
        if let Some(time) = self.update_anim_track(cx, time) {
            if let Some(track_index) = self.find_track_index(bind_id) {
                if let Track::Vec2{keys, cut_init, ease, ..} = &mut self.current.as_mut().unwrap().tracks[track_index] {
                    ret = Track::compute_track_vec2(time, keys, cut_init, last, ease);
                }
            }
        }
        self.set_last_vec2(bind_id, ret);
        return ret
    }
    
    pub fn last_vec2(&self, _cx: &Cx, bind_id: LiveItemId) -> Vec2 {
        Self::_last_vec2(bind_id, &self.last_values)
    }
    
    pub fn _last_vec2(live_item_id: LiveItemId, last_values: &Vec<(LiveItemId, AnimLastValue)>) -> Vec2 {
        if let Some((_, value)) = last_values.iter().find( | v | v.0 == live_item_id) {
            if let AnimLastValue::Vec2(value) = value {
                return *value
            }
        }
        return Vec2::default()
    }
    
    pub fn set_last_vec2(&mut self, live_item_id: LiveItemId, value: Vec2) {
        Self::_set_last_vec2(live_item_id, value, &mut self.last_values);
    }
    
    pub fn _set_last_vec2(live_item_id: LiveItemId, value: Vec2, last_values: &mut Vec<(LiveItemId, AnimLastValue)>) {
        if let Some((_, last)) = last_values.iter_mut().find( | v | v.0 == live_item_id) {
            *last = AnimLastValue::Vec2(value);
        }
        else {
            last_values.push((live_item_id, AnimLastValue::Vec2(value)))
        }
    }
    
    pub fn calc_vec3(&mut self, cx: &mut Cx, live_item_id: LiveItemId, time: f64) -> Vec3 {
        let last = Self::_last_vec3(live_item_id, &self.last_values);
        let mut ret = last;
        if let Some(time) = self.update_anim_track(cx, time) {
            if let Some(track_index) = self.find_track_index(live_item_id) {
                if let Track::Vec3{keys, cut_init, ease, ..} = &mut self.current.as_mut().unwrap().tracks[track_index] {
                    ret = Track::compute_track_vec3(time, keys, cut_init, last, ease);
                }
            }
        }
        self.set_last_vec3(live_item_id, ret);
        return ret
    }
    
    pub fn last_vec3(&self, _cx: &Cx, live_item_id: LiveItemId) -> Vec3 {
        Self::_last_vec3(live_item_id, &self.last_values)
    }
    
    pub fn _last_vec3(live_item_id: LiveItemId, last_values: &Vec<(LiveItemId, AnimLastValue)>) -> Vec3 {
        if let Some((_, value)) = last_values.iter().find( | v | v.0 == live_item_id) {
            if let AnimLastValue::Vec3(value) = value {
                return *value
            }
        }
        return Vec3::default()
    }
    
    pub fn set_last_vec3(&mut self, live_item_id: LiveItemId, value: Vec3) {
        Self::_set_last_vec3(live_item_id, value, &mut self.last_values);
    }
    
    pub fn _set_last_vec3(live_item_id: LiveItemId, value: Vec3, last_values: &mut Vec<(LiveItemId, AnimLastValue)>) {
        if let Some((_, last)) = last_values.iter_mut().find( | v | v.0 == live_item_id) {
            *last = AnimLastValue::Vec3(value);
        }
        else {
            last_values.push((live_item_id, AnimLastValue::Vec3(value)))
        }
    }
    
    pub fn calc_vec4(&mut self, cx: &mut Cx, live_item_id: LiveItemId, time: f64) -> Vec4 {
        let last = Self::_last_vec4(live_item_id, &self.last_values);
        let mut ret = last;
        if let Some(time) = self.update_anim_track(cx, time) {
            if let Some(track_index) = self.find_track_index(live_item_id) {
                if let Track::Vec4{keys, cut_init, ease, ..} = &mut self.current.as_mut().unwrap().tracks[track_index] {
                    ret = Track::compute_track_vec4(time, keys, cut_init, last, ease);
                }
            }
        }
        self.set_last_vec4(live_item_id, ret);
        return ret
    }
    
    pub fn last_vec4(&self, _cx: &Cx, live_item_id: LiveItemId) -> Vec4 {
        Self::_last_vec4(live_item_id, &self.last_values)
    }
    
    pub fn _last_vec4(live_item_id: LiveItemId, last_values: &Vec<(LiveItemId, AnimLastValue)>) -> Vec4 {
        if let Some((_, value)) = last_values.iter().find( | v | v.0 == live_item_id) {
            if let AnimLastValue::Vec4(value) = value {
                return *value
            }
        }
        return Vec4::default()
    }
    
    pub fn set_last_vec4(&mut self, live_item_id: LiveItemId, value: Vec4) {
        Self::_set_last_vec4(live_item_id, value, &mut self.last_values);
    }
    
    pub fn _set_last_vec4(live_item_id: LiveItemId, value: Vec4, last_values: &mut Vec<(LiveItemId, AnimLastValue)>) {
        if let Some((_, last)) = last_values.iter_mut().find( | v | v.0 == live_item_id) {
            *last = AnimLastValue::Vec4(value);
        }
        else {
            last_values.push((live_item_id, AnimLastValue::Vec4(value)))
        }
    }
    
    pub fn calc_color(&mut self, cx: &mut Cx, live_item_id: LiveItemId, time: f64) -> Color {
        if let Some(time) = self.update_anim_track(cx, time) {
            if let Some(track_index) = self.find_track_index(live_item_id) {
                if let Track::Color{keys, cut_init, ease, ..} = &mut self.current.as_mut().unwrap().tracks[track_index] {
                    let last = Self::_last_color(live_item_id, &self.last_values);
                    let ret = Track::compute_track_color(time, keys, cut_init, last, ease);
                    self.set_last_color(live_item_id, ret);
                    return ret
                }
            }
        }
        
        return Color::default();
    }
    
    pub fn last_color(&self, _cx: &Cx, live_item_id: LiveItemId) -> Color {
        if let Some((_, value)) = self.last_values.iter().find( | v | v.0 == live_item_id) {
            if let AnimLastValue::Color(value) = value {
                return *value
            }
        }
        Color::default()
    }
    
    pub fn _last_color(live_item_id: LiveItemId, last_values: &Vec<(LiveItemId, AnimLastValue)>) -> Color {
        if let Some((_, value)) = last_values.iter().find( | v | v.0 == live_item_id) {
            if let AnimLastValue::Color(value) = value {
                return *value
            }
        }
        
        return Color::default()
    }
    
    pub fn set_last_color(&mut self, live_item_id: LiveItemId, value: Color) {
        Self::_set_last_color(live_item_id, value, &mut self.last_values);
    }
    
    pub fn _set_last_color(live_item_id: LiveItemId, value: Color, last_values: &mut Vec<(LiveItemId, AnimLastValue)>) {
        if let Some((_, last)) = last_values.iter_mut().find( | v | v.0 == live_item_id) {
            *last = AnimLastValue::Color(value)
        }
        else {
            last_values.push((live_item_id, AnimLastValue::Color(value)))
        }
    }
    
    pub fn last_area(&mut self, _cx: &mut Cx, _area: Area, _time: f64) {
        
    }
    
    pub fn calc_area(&mut self, cx: &mut Cx, area: Area, time: f64) {
        
        if let Some(time) = self.update_anim_track(cx, time) {
            
            for track_index in 0..self.current.as_ref().unwrap().tracks.len() {
                //if let Some((time, track_index)) = self.fetch_calc_track(cx, ident, time) {
                match &mut self.current.as_mut().unwrap().tracks[track_index] {
                    Track::Color{bind_to, keys, cut_init, ease} => {
                        let init = Self::_last_color(*bind_to, &self.last_values);
                        let ret = Track::compute_track_color(time, keys, cut_init, init, ease);
                        Self::_set_last_color(*bind_to, ret, &mut self.last_values);
                        area.write_color(cx, *bind_to, ret);
                    },
                    Track::Vec4{bind_to, keys, cut_init, ease} => {
                        let init = Self::_last_vec4(*bind_to, &self.last_values);
                        let ret = Track::compute_track_vec4(time, keys, cut_init, init, ease);
                        Self::_set_last_vec4(*bind_to, ret, &mut self.last_values);
                        area.write_vec4(cx, *bind_to, ret);
                    },
                    Track::Vec3{bind_to, keys, cut_init, ease} => {
                        let init = Self::_last_vec3(*bind_to, &self.last_values);
                        let ret = Track::compute_track_vec3(time, keys, cut_init, init, ease);
                        Self::_set_last_vec3(*bind_to, ret, &mut self.last_values);
                        area.write_vec3(cx, *bind_to, ret);
                    },
                    Track::Vec2{bind_to, keys, cut_init, ease} => {
                        let init = Self::_last_vec2(*bind_to, &self.last_values);
                        let ret = Track::compute_track_vec2(time, keys, cut_init, init, ease);
                        Self::_set_last_vec2(*bind_to, ret, &mut self.last_values);
                        area.write_vec2(cx, *bind_to, ret);
                    },
                    Track::Float{bind_to, keys, cut_init, ease} => {
                        let init = Self::_last_float(*bind_to, &self.last_values);
                        let ret = Track::compute_track_float(time, keys, cut_init, init, ease);
                        Self::_set_last_float(*bind_to, ret, &mut self.last_values);
                        area.write_float(cx, *bind_to, ret);
                    }
                };
            }
            //}
        }
    }
}

