#ifndef ROCKET_H
#define ROCKET_H

#include "environment.h"

typedef struct {
    double Ve[3];  // 速度 (m/s)
    double Xe[3];  // 位置 (m)
    double acc[3]; // 加速度 (m/s^2)
    double mass;   // 質量 (kg)
    double thrust; // 推力 (N)
    double thrust_duration; // 推力の持続時間 (秒)
} Rocket;

void compute_acceleration(Rocket *r, Environment *env);
void update_rocket_leapfrog(Rocket *r, Environment *env, double dt);
double compute_thrust(double time, Rocket *r);

#endif
