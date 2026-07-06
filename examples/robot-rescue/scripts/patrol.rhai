// Drone that patrols horizontally and spins faster when the player is near.
// API: x(), move_by(), set_rotation(), distance_to(), get_var/set_var, log()...
// Full reference: sdk/aigs-format/scripting-api.json (or `aigs script-api`).

fn on_start() {
    set_var("dir", 1.0);
    log("drone online");
}

fn on_update(dt) {
    // Walk back and forth between x = -250 and x = 250. "dir" is this
    // instance's persistent state: fn bodies don't see outer variables
    // between calls, so get_var/set_var is how a script remembers things
    // across ticks (see on_start above).
    let dir = get_var("dir");
    if x() > 250.0 { dir = -1.0; }
    if x() < -250.0 { dir = 1.0; }
    set_var("dir", dir);
    move_by(dir * 140.0 * dt, 0.0);

    // Spin, faster when the player gets close.
    let near = distance_to("player") < 150.0;
    let speed = if near { 540.0 } else { 90.0 };
    set_rotation(rotation() + speed * dt);
}

fn on_destroy() {
    log("drone offline");
}
