// Drone that patrols horizontally and spins faster when the player is near.
// API: x(), y(), move_by(), set_rotation(), distance_to(), log()...

fn on_start() {
    log("drone online");
}

fn on_update(dt) {
    // Walk back and forth between x = -250 and x = 250.
    if !("dir" in this) { this.dir = 1.0; }
    if x() > 250.0 { this.dir = -1.0; }
    if x() < -250.0 { this.dir = 1.0; }
    move_by(this.dir * 140.0 * dt, 0.0);

    // Spin, faster when the player gets close.
    let near = distance_to("player") < 150.0;
    let speed = if near { 540.0 } else { 90.0 };
    set_rotation(rotation() + speed * dt);
}
