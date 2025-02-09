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

void update_rocket_leapfrog(Rocket *r, Environment *env, double dt) {
    for (int i = 0; i < 3; i++) r->Ve[i] += 0.5 * r->acc[i] * dt;
    for (int i = 0; i < 3; i++) r->Xe[i] += r->Ve[i] * dt;

    compute_acceleration(r, env);

    for (int i = 0; i < 3; i++) r->Ve[i] += 0.5 * r->acc[i] * dt;
}
