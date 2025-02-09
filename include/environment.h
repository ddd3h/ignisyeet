#ifndef ENVIRONMENT_H
#define ENVIRONMENT_H

typedef struct {
    double wind[3];     // 風速 (m/s)
    double air_density; // 大気密度 (kg/m^3)
    double gravity;     // 重力加速度 (m/s^2)
    double drag_coef;   // 抗力係数
    double area;        // 断面積 (m^2)
    double dt;          // 時間刻み (秒)
} Environment;

#endif
