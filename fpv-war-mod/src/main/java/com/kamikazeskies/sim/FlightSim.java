package com.kamikazeskies.sim;

/**
 * Rigid-body quadcopter flight simulation with a rate PID controller, angle-mode attitude controller,
 * X-layout motor mixer (with saturation handling), motor lag, gravity and quadratic drag.
 *
 * <p>Pure Java, deterministic, no Minecraft dependency. Call {@link #step} at a fixed dt (e.g. 1/80 s).
 * The caller applies the returned displacement to the world (with collision) and reports what actually
 * happened through {@link #commit}.
 *
 * <p>Frames: world = Minecraft (+X east, +Y up, +Z south). Body: +Y up/thrust, +Z nose, -X pilot-right.
 * Body angular velocity uses the right-hand rule about the body axes: +x = nose down, +z = roll right,
 * +y = yaw left.
 */
public final class FlightSim {
    // Motor order: front-right, front-left, rear-right, rear-left.
    private static final double[] A = {1, -1, 1, -1};   // +1 = pilot-right side
    private static final double[] B = {1, 1, -1, -1};   // +1 = front
    private static final double[] S = {1, -1, -1, 1};   // spin direction (yaw reaction torque sign)
    private static final double RELAX_ERR = 4.0;
    private static final double MAX_I_ACCEL = 80.0;     // rad/s^2 integrator clamp

    private final DroneParams p;
    private Vec3d pos = Vec3d.ZERO;
    private Vec3d vel = Vec3d.ZERO;
    private Quat q = Quat.IDENTITY;
    private Vec3d omega = Vec3d.ZERO;                    // body frame, rad/s
    private final double[] thrust = new double[4];       // actual per-motor thrust, N
    private Vec3d iTerm = Vec3d.ZERO;                    // rate integrator, rad/s^2
    private Vec3d heading = new Vec3d(0, 0, 1);          // last horizontal heading (angle mode fallback)
    private double impactSpeed;                          // largest speed absorbed by the last commit, m/s
    private boolean wasArmed;
    private boolean saturated;                           // mixer ran out of authority last step (anti-windup)

    public FlightSim(DroneParams params) { this.p = params; }

    // ---- state access -------------------------------------------------------------------------------
    public DroneParams params() { return p; }
    public Vec3d position() { return pos; }
    public Vec3d velocity() { return vel; }
    public Quat orientation() { return q; }
    public Vec3d angularVelocity() { return omega; }
    public double impactSpeed() { return impactSpeed; }
    public double motorThrust(int i) { return thrust[i]; }
    /** Average motor output 0..1 (for sound and OSD). */
    public double averageMotorFraction() {
        return (thrust[0] + thrust[1] + thrust[2] + thrust[3]) / (4 * p.maxThrustPerMotor());
    }
    public double speed() { return vel.length(); }

    public void setPosition(Vec3d position) { this.pos = position; }

    /**
     * Resting/rolling on the ground: friction on horizontal velocity, spin damping and gentle self-leveling
     * so a landed drone neither slides forever nor stays balanced on an edge.
     */
    public void groundContact(double horizontalFriction) {
        vel = new Vec3d(vel.x() * horizontalFriction, vel.y(), vel.z() * horizontalFriction);
        omega = omega.scale(0.85);
        q = Quat.nlerp(q, q.leveled(), 0.2);
    }

    public void setState(Vec3d position, Vec3d velocity, Quat orientation, Vec3d angularVelocity) {
        this.pos = position; this.vel = velocity; this.q = orientation.normalized(); this.omega = angularVelocity;
    }

    /** Test/spawn helper: motors already spinning at the hover point. */
    public void spinUpToHover() {
        for (int i = 0; i < 4; i++) thrust[i] = p.mass() * DroneParams.GRAVITY / 4.0;
        wasArmed = true;
    }

    // ---- simulation ---------------------------------------------------------------------------------
    /** Advance one fixed step. Position is NOT changed; apply the returned displacement via {@link #commit}. */
    public Vec3d step(FlightInput in, double dt) {
        impactSpeed = 0;
        updateHeading();

        double[] cmd = new double[4];
        if (in.armed()) {
            Vec3d torque = controlTorque(in, dt);
            mix(in, torque, cmd);
        } else {
            iTerm = Vec3d.ZERO;
            saturated = false;
        }
        wasArmed = in.armed();

        // motors: first-order lag toward the command
        double k = 1.0 - Math.exp(-dt / p.motorTimeConstant());
        for (int i = 0; i < 4; i++) thrust[i] += (cmd[i] - thrust[i]) * k;

        // forces
        double total = thrust[0] + thrust[1] + thrust[2] + thrust[3];
        Vec3d thrustWorld = q.rotate(new Vec3d(0, total, 0));
        Vec3d gravity = new Vec3d(0, -p.mass() * DroneParams.GRAVITY, 0);
        Vec3d drag = vel.scale(-p.dragCoefficient() * vel.length());
        Vec3d accel = thrustWorld.add(gravity).add(drag).scale(1.0 / p.mass());
        vel = vel.add(accel.scale(dt));

        // torques (body frame)
        double d = p.armHalfSpan(), kyaw = p.yawTorqueRatio();
        double tx = 0, tz = 0, ty = 0;
        for (int i = 0; i < 4; i++) {
            tx += -d * B[i] * thrust[i];
            tz += -d * A[i] * thrust[i];
            ty += kyaw * S[i] * thrust[i];
        }
        double ix = p.inertiaPitchRoll(), iy = p.inertiaYaw(), iz = p.inertiaPitchRoll();
        Vec3d iw = new Vec3d(ix * omega.x(), iy * omega.y(), iz * omega.z());
        Vec3d gyro = omega.cross(iw);
        Vec3d alpha = new Vec3d((tx - gyro.x()) / ix, (ty - gyro.y()) / iy, (tz - gyro.z()) / iz);
        omega = omega.add(alpha.scale(dt));
        q = q.integrate(omega, dt);

        return vel.scale(dt);
    }

    /**
     * Apply the displacement the world actually allowed. Velocity components that were blocked are removed
     * and the absorbed speed is recorded in {@link #impactSpeed()}.
     */
    public void commit(Vec3d requested, Vec3d actual) {
        double eps = 1e-7;
        double vx = vel.x(), vy = vel.y(), vz = vel.z(), imp = 0;
        if (Math.abs(actual.x()) < Math.abs(requested.x()) - eps) { imp = Math.max(imp, Math.abs(vx)); vx = 0; }
        if (Math.abs(actual.y()) < Math.abs(requested.y()) - eps) { imp = Math.max(imp, Math.abs(vy)); vy = 0; }
        if (Math.abs(actual.z()) < Math.abs(requested.z()) - eps) { imp = Math.max(imp, Math.abs(vz)); vz = 0; }
        vel = new Vec3d(vx, vy, vz);
        pos = pos.add(actual);
        impactSpeed = Math.max(impactSpeed, imp);
    }

    /** Free flight (no world): step and apply the full displacement. */
    public void stepFree(FlightInput in, double dt) {
        Vec3d d = step(in, dt);
        commit(d, d);
    }

    // ---- control ------------------------------------------------------------------------------------
    private void updateHeading() {
        Vec3d f = q.bodyForward();
        Vec3d h = new Vec3d(f.x(), 0, f.z());
        if (h.length() > 0.2) heading = h.normalized();
    }

    /** Desired body angular velocity (rad/s). */
    Vec3d desiredRates(FlightInput in) {
        double wxA = Math.toRadians(p.rollPitchRates().rate(in.pitch()));
        double wzA = Math.toRadians(p.rollPitchRates().rate(in.roll()));
        double wy = -Math.toRadians(p.yawRates().rate(in.yaw()));
        Vec3d acro = new Vec3d(wxA, wy, wzA);
        if (in.assist() <= 0) return acro;

        // angle mode: stick selects a target tilt in the heading frame
        double maxA = Math.toRadians(p.maxAngleDeg());
        Vec3d hf = heading;
        Vec3d hr = new Vec3d(-hf.z(), 0, hf.x());
        Vec3d v = hf.scale(Math.sin(in.pitch() * maxA)).add(hr.scale(Math.sin(in.roll() * maxA)));
        double vy = Math.sqrt(Math.max(0, 1 - v.lengthSq()));
        Vec3d targetUp = new Vec3d(v.x(), vy, v.z()).normalized();
        Vec3d err = q.unrotate(q.bodyUp().cross(targetUp));         // axis*sin(angle), body frame
        double cap = Math.toRadians(p.maxAngleRateDegS());
        double ex = clamp(p.angleGain() * err.x(), -cap, cap);
        double ez = clamp(p.angleGain() * err.z(), -cap, cap);
        Vec3d angle = new Vec3d(ex, wy, ez);

        double a = in.assist();
        return acro.scale(1 - a).add(angle.scale(a));
    }

    /** Rate PI controller -> desired body torque (N*m). */
    private Vec3d controlTorque(FlightInput in, double dt) {
        Vec3d want = desiredRates(in);
        Vec3d err = want.sub(omega);
        if (saturated) {
            iTerm = iTerm.scale(0.97);   // no authority left: freeze growth and bleed off (anti-windup)
        } else {
            // "I-term relax": integrate fully only when the rate error is small, so fast stick moves
            // do not leave a large integrator residue that would take seconds to unwind.
            iTerm = new Vec3d(
                    clamp(iTerm.x() + relax(err.x()) * p.rateIGain() * err.x() * dt, -MAX_I_ACCEL, MAX_I_ACCEL),
                    clamp(iTerm.y() + relax(err.y()) * p.yawIGain() * err.y() * dt, -MAX_I_ACCEL, MAX_I_ACCEL),
                    clamp(iTerm.z() + relax(err.z()) * p.rateIGain() * err.z() * dt, -MAX_I_ACCEL, MAX_I_ACCEL));
        }
        double ax = p.ratePGain() * err.x() + iTerm.x();
        double ay = p.yawPGain() * err.y() + iTerm.y();
        double az = p.ratePGain() * err.z() + iTerm.z();
        return new Vec3d(ax * p.inertiaPitchRoll(), ay * p.inertiaYaw(), az * p.inertiaPitchRoll());
    }

    /** X-quad mixer with saturation handling. Writes per-motor thrust commands (N). */
    private void mix(FlightInput in, Vec3d torque, double[] out) {
        double tmax = p.maxThrustPerMotor();
        double collective = 4 * tmax * p.throttleToThrustFraction(in.throttle());
        if (in.throttle() < 0.03) { // idle: no airmode, motors stop
            saturated = true;
            java.util.Arrays.fill(out, 0);
            return;
        }
        double d = p.armHalfSpan(), k = p.yawTorqueRatio();
        double[] diff = new double[4];
        double maxD = -1e9, minD = 1e9;
        for (int i = 0; i < 4; i++) {
            diff[i] = -torque.x() * B[i] / (4 * d) - torque.z() * A[i] / (4 * d) + torque.y() * S[i] / (4 * k);
            maxD = Math.max(maxD, diff[i]);
            minD = Math.min(minD, diff[i]);
        }
        saturated = false;
        if (maxD - minD > tmax) { // not enough authority: scale differentials down
            saturated = true;
            double s = tmax / (maxD - minD);
            for (int i = 0; i < 4; i++) diff[i] *= s;
            maxD *= s; minD *= s;
        }
        double base = clamp(collective / 4.0, -minD, tmax - maxD);   // "airmode": shift base to keep authority
        for (int i = 0; i < 4; i++) {
            double t = base + diff[i];
            if (t < 0 || t > tmax) saturated = true;
            out[i] = clamp(t, 0, tmax);
        }
    }

    /** 1 at zero rate error, falling to 0 at {@value #RELAX_ERR} rad/s (~230 deg/s) and beyond. */
    private static double relax(double err) { return Math.max(0.0, 1.0 - Math.abs(err) / RELAX_ERR); }

    private static double clamp(double v, double lo, double hi) { return v < lo ? lo : Math.min(v, hi); }
}
