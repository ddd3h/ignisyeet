#ifndef ROCKET_H
#define ROCKET_H

#include "environment.hpp"
#include <array>

class Rocket {
public:
    // 位置・速度・加速度
    std::array<double, 3> Ve;  // 速度 (m/s)
    std::array<double, 3> Xe;  // 位置 (m)
    std::array<double, 3> acc; // 加速度 (m/s^2)
    std::array<double, 3> omg; // 角速度 (rad/s)
    std::array<double, 4> q;   // クォータニオン（姿勢）

    // 機体特性
    double mass;
    double thrust;
    double thrust_duration;
    double drag_coefficient;
    double length;    // 🔹 **機体の長さを追加**
    double diameter;  // 🔹 **機体の直径を追加**

    // ランチャーの初期設定
    double launcher_azimuth;
    double launcher_elevation;

    Rocket();
    void update(Environment& env, double dt);
    double compute_thrust(double time);
};

#endif
