// Tamagotchi brain (milestone M13): hunger/happiness/health decay over
// time — even while the game is closed, thanks to elapsed_since_save() —
// and keyboard actions feed it back up. All state lives in get_var/set_var,
// which `aigs run` autosaves to disk every ~10 seconds.
//
// Keys: 1 = feed, 2 = play, 3 = medicine.
// Full API reference: sdk/aigs-format/scripting-api.json (or `aigs script-api`).

fn clamp(v, lo, hi) {
    if v < lo { return lo; }
    if v > hi { return hi; }
    v
}

fn apply_decay(dt) {
    if get_var("health") <= 0.0 {
        return;
    }
    set_var("fullness", clamp(get_var("fullness") - dt * 1.6, 0.0, 100.0));
    set_var("happiness", clamp(get_var("happiness") - dt * 1.1, 0.0, 100.0));
    let neglected = get_var("fullness") <= 0.0 || get_var("happiness") <= 0.0;
    if neglected {
        set_var("health", clamp(get_var("health") - dt * 6.0, 0.0, 100.0));
    } else {
        set_var("health", clamp(get_var("health") + dt * 2.0, 0.0, 100.0));
    }
    set_var("age", get_var("age") + dt);
}

fn on_start() {
    if get_var("initialized") < 0.5 {
        set_var("initialized", 1.0);
        set_var("fullness", 80.0);
        set_var("happiness", 80.0);
        set_var("health", 100.0);
        set_var("age", 0.0);
        log("a new pet hatches!");
    } else {
        let offline = elapsed_since_save();
        if offline > 1.0 {
            log("welcome back after being away");
            apply_decay(offline);
        }
    }
}

fn feed() {
    set_var("fullness", clamp(get_var("fullness") + 30.0, 0.0, 100.0));
    set_var("eat_flash", 0.6);
    play_sound("chomp");
    play_animation("feed-pulse");
}

fn play_with_pet() {
    set_var("happiness", clamp(get_var("happiness") + 25.0, 0.0, 100.0));
    set_var("happy_flash", 0.6);
    play_sound("giggle");
    play_animation("play-pulse");
}

fn give_medicine() {
    if get_var("health") < 100.0 {
        set_var("health", clamp(get_var("health") + 35.0, 0.0, 100.0));
        play_sound("heal");
        play_animation("medicine-pulse");
    }
}

fn update_visual(dt) {
    if get_var("health") <= 0.0 {
        set_frame(5.0); // fainted
        return;
    }
    if get_var("eat_flash") > 0.0 {
        set_frame(2.0); // eating
        return;
    }
    if get_var("happy_flash") > 0.0 {
        set_frame(3.0); // happy
        return;
    }
    if get_var("fullness") < 30.0 || get_var("happiness") < 30.0 {
        set_frame(4.0); // sad / hungry
        return;
    }
    // Idle blink cycle.
    if get_var("blink_timer") <= 0.0 {
        set_var("blink_timer", 2.5);
        if get_var("blink_state") > 0.5 {
            set_var("blink_state", 0.0);
        } else {
            set_var("blink_state", 1.0);
        }
    } else {
        set_var("blink_timer", get_var("blink_timer") - dt);
    }
    set_frame(get_var("blink_state"));
}

fn on_update(dt) {
    apply_decay(dt);

    if get_var("eat_flash") > 0.0 { set_var("eat_flash", get_var("eat_flash") - dt); }
    if get_var("happy_flash") > 0.0 { set_var("happy_flash", get_var("happy_flash") - dt); }

    if key_pressed("Digit1") { feed(); }
    if key_pressed("Digit2") { play_with_pet(); }
    if key_pressed("Digit3") { give_medicine(); }

    // Grows from baby (0.6x) to adult (1.0x) over its first ~3 minutes.
    let scale = clamp(0.6 + get_var("age") / 200.0, 0.6, 1.0);
    set_scale(scale, scale);

    update_visual(dt);
}
