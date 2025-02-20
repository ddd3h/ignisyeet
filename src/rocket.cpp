#include "rocket.hpp"
#include <cmath>

Rocket::Rocket() : mass(0), thrust(0), thrust_duration(0), drag_coefficient(0), length(0), diameter(0) {
    Ve.fill(0.0);
    Xe.fill(0.0);
    acc.fill(0.0);
    omg.fill(0.0);
    q = {1.0, 0.0, 0.0, 0.0};
}

double Rocket::compute_thrust(double time) {
    return (time < thrust_duration) ? thrust : 0.0;
}

void Rocket::update(Environment& env, double dt) {
    double force[3] = {0, 0, thrust - mass * env.gravity}; 
    double drag = 0.5 * env.air_density * env.area * drag_coefficient * std::pow(Ve[2], 2);
    force[2] -= drag;

    for (int i = 0; i < 3; i++) acc[i] = force[i] / mass;

    for (int i = 0; i < 3; i++) Ve[i] += 0.5 * acc[i] * dt;
    for (int i = 0; i < 3; i++) Xe[i] += Ve[i] * dt;

    for (int i = 0; i < 3; i++) Ve[i] += 0.5 * acc[i] * dt;
}
