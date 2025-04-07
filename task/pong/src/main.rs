// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![no_std]
#![no_main]

use userlib::*;

#[cfg(feature = "led")]
task_slot!(USER_LEDS, user_leds);

#[cfg(feature = "led")]
pub fn toggle_leds(user_leds: drv_user_leds_api::UserLeds) {

    loop {
        user_leds.toggle();
        sys_delay(100);
    }
}

#[export_name = "main"]
pub fn main() -> ! {
    const INTERVAL: u64 = 500;

    let mut response: u32 = 0;

    #[cfg(feature = "led")]
    let user_leds = drv_user_leds_api::UserLeds::from(USER_LEDS.get_task_id());

    let mut current = 0;
    let mut msg = [0; 16];
    let mut dl = INTERVAL;
    sys_set_timer(Some(dl), notifications::TIMER_MASK);
    loop {
        let msginfo = sys_recv_open(&mut msg, notifications::TIMER_MASK);

        if msginfo.sender != TaskId::KERNEL {
            // We'll just assume this is a ping message and reply.
            sys_reply(msginfo.sender, response, &[]);
            response += 1;
        } else {
            // This is a notification message. We've only got one notification
            // enabled, so we know full well which it is without looking.
            dl += INTERVAL;
            sys_set_timer(Some(dl), notifications::TIMER_MASK);

            // Toggle the current LED -- and if we've run out, start over
            loop {                
                #[cfg(feature = "led")]
                toggle_leds(user_leds);
            }
        }
    }
}

include!(concat!(env!("OUT_DIR"), "/notifications.rs"));
