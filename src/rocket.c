#include "rocket.h"
#include <math.h>

double compute_thrust(double time, Rocket *r) {
    return (time < r->thrust_duration) ? r->thrust : 0.0;
}

void compute_acceleration(Rocket *r, Environment *env) {
    double force[3] = {0, 0, r->thrust - r->mass * env->gravity}; 

    double drag = 0.5 * env->air_density * env->area * env->drag_coef * pow(r->Ve[2], 2);
    force[2] -= drag;

    for (int i = 0; i < 3; i++) {
        r->acc[i] = force[i] / r->mass;
    }
}


void update_angular_velocity(Rocket *r, double dt) {
    double torque[3] = {0.0, 0.0, 0.0}; // モーメント（推力や空気力）
    double I[3] = {10.0, 15.0, 20.0}; // 簡易的な慣性テンソル（仮）

    for (int i = 0; i < 3; i++) {
        r->omg[i] += (torque[i] / I[i]) * dt; // 角速度の更新
    }
}

void update_orientation(Rocket *r, double dt) {
    double q_dot[4];
    q_dot[0] = 0.5 * (-r->q[1] * r->omg[0] - r->q[2] * r->omg[1] - r->q[3] * r->omg[2]);
    q_dot[1] = 0.5 * ( r->q[0] * r->omg[0] + r->q[2] * r->omg[2] - r->q[3] * r->omg[1]);
    q_dot[2] = 0.5 * ( r->q[0] * r->omg[1] - r->q[1] * r->omg[2] + r->q[3] * r->omg[0]);
    q_dot[3] = 0.5 * ( r->q[0] * r->omg[2] + r->q[1] * r->omg[1] - r->q[2] * r->omg[0]);

    for (int i = 0; i < 4; i++) {
        r->q[i] += q_dot[i] * dt; // クォータニオンの更新
    }

    // 正規化
    double norm = sqrt(r->q[0] * r->q[0] + r->q[1] * r->q[1] +
                       r->q[2] * r->q[2] + r->q[3] * r->q[3]);
    for (int i = 0; i < 4; i++) {
        r->q[i] /= norm;
    }
}

void update_rocket_leapfrog(Rocket *r, Environment *env, double dt) {
    for (int i = 0; i < 3; i++) r->Ve[i] += 0.5 * r->acc[i] * dt;
    for (int i = 0; i < 3; i++) r->Xe[i] += r->Ve[i] * dt;

    // 角速度と姿勢の更新
    update_angular_velocity(r, dt);
    update_orientation(r, dt);

    compute_acceleration(r, env);
    for (int i = 0; i < 3; i++) r->Ve[i] += 0.5 * r->acc[i] * dt;
}