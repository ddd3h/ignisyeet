#pragma once

#include "vector3d.hpp"

namespace ignis {

using namespace IgnisYeet::Physics;  // Use Physics namespace

/**
 * @brief 重力モデルの基底クラス
 * 
 * 段階的な重力精度レベルを提供するための抽象基底クラス。
 * Level 1: 一様重力場
 * Level 2: 高度依存重力  
 * Level 3: 楕円体地球+コリオリ力
 */
class GravityModel {
public:
    virtual ~GravityModel() = default;
    
    /**
     * @brief 指定位置・速度での重力加速度を計算
     * @param position 位置ベクトル [m] (地心固定座標系)
     * @param velocity 速度ベクトル [m/s] (Level 3でコリオリ力計算に使用)
     * @return 重力加速度ベクトル [m/s²]
     */
    virtual Vector3D calculate_gravity(const Vector3D& position, 
                                     const Vector3D& velocity) const = 0;
    
    /**
     * @brief 重力モデルのレベルを取得
     * @return 重力精度レベル (1-3)
     */
    virtual int get_level() const = 0;
    
    /**
     * @brief 重力モデルの説明を取得
     * @return モデルの説明文字列
     */
    virtual std::string get_description() const = 0;
};

/**
 * @brief Level 1: 一様重力場モデル
 * 
 * 地表付近での定数重力加速度を想定。
 * 最も単純で計算コストが最小。
 * 低高度ロケット（<10km）に適用。
 */
class UniformGravity : public GravityModel {
private:
    double g0_;  ///< 地表重力加速度 [m/s²]
    
public:
    /**
     * @brief コンストラクタ
     * @param g0 地表重力加速度 [m/s²] (デフォルト: 9.81)
     */
    explicit UniformGravity(double g0 = 9.81) : g0_(g0) {}
    
    Vector3D calculate_gravity(const Vector3D& position, 
                             const Vector3D& velocity) const override {
        // 一様重力場: 位置・速度に関係なく一定
        return Vector3D(0.0, 0.0, -g0_);
    }
    
    int get_level() const override { return 1; }
    
    std::string get_description() const override {
        return "Uniform gravity field (Level 1)";
    }
    
    /**
     * @brief 重力加速度値を取得
     * @return 地表重力加速度 [m/s²]
     */
    double get_g0() const { return g0_; }
    
    /**
     * @brief 重力加速度値を設定
     * @param g0 地表重力加速度 [m/s²]
     */
    void set_g0(double g0) { g0_ = g0; }
};

/**
 * @brief Level 2: 高度依存重力モデル
 * 
 * 地球中心からの距離に応じた重力変化を考慮。
 * 式: g(r) = g0 * (R_earth / r)²
 * 高高度ロケット（>10km）に適用。
 */
class AltitudeDependentGravity : public GravityModel {
private:
    double g0_;           ///< 地表重力加速度 [m/s²]
    double earth_radius_; ///< 地球半径 [m]
    
public:
    /**
     * @brief コンストラクタ
     * @param g0 地表重力加速度 [m/s²] (デフォルト: 9.81)
     * @param earth_radius 地球半径 [m] (デフォルト: 6371000.0)
     */
    explicit AltitudeDependentGravity(double g0 = 9.81, 
                                    double earth_radius = 6371000.0)
        : g0_(g0), earth_radius_(earth_radius) {}
    
    Vector3D calculate_gravity(const Vector3D& position, 
                             const Vector3D& velocity) const override {
        // 地心からの距離を計算
        double r = position.magnitude();
        
        // 地球半径より小さい場合はエラー回避
        if (r < earth_radius_ * 0.99) {
            r = earth_radius_;
        }
        
        // 重力の大きさ: g(r) = g0 * (R_earth / r)²
        double g_magnitude = g0_ * (earth_radius_ / r) * (earth_radius_ / r);
        
        // 重力の方向: 地心に向かう単位ベクトル
        Vector3D direction = position.normalized() * (-1.0);
        
        return direction * g_magnitude;
    }
    
    int get_level() const override { return 2; }
    
    std::string get_description() const override {
        return "Altitude-dependent gravity (Level 2)";
    }
    
    /**
     * @brief パラメータを取得
     */
    double get_g0() const { return g0_; }
    double get_earth_radius() const { return earth_radius_; }
    
    /**
     * @brief パラメータを設定
     */
    void set_g0(double g0) { g0_ = g0; }
    void set_earth_radius(double radius) { earth_radius_ = radius; }
};

/**
 * @brief Level 3: 楕円体地球+コリオリ力モデル
 * 
 * 最高精度の重力モデル。以下を考慮:
 * - 地球の楕円体形状
 * - 地球自転によるコリオリ力・遠心力
 * 衛星ミッション・高精度軌道計算に適用。
 */
class RotatingEarthGravity : public GravityModel {
private:
    double g0_;               ///< 地表重力加速度 [m/s²]
    double earth_radius_;     ///< 地球半径 [m]
    double rotation_rate_;    ///< 地球自転角速度 [rad/s]
    
public:
    /**
     * @brief コンストラクタ
     * @param g0 地表重力加速度 [m/s²] (デフォルト: 9.81)
     * @param earth_radius 地球半径 [m] (デフォルト: 6371000.0)
     * @param rotation_rate 地球自転角速度 [rad/s] (デフォルト: 7.2921159e-5)
     */
    explicit RotatingEarthGravity(double g0 = 9.81,
                                double earth_radius = 6371000.0,
                                double rotation_rate = 7.2921159e-5)
        : g0_(g0), earth_radius_(earth_radius), rotation_rate_(rotation_rate) {}
    
    Vector3D calculate_gravity(const Vector3D& position, 
                             const Vector3D& velocity) const override {
        // 基本重力項（高度依存）
        double r = position.magnitude();
        if (r < earth_radius_ * 0.99) {
            r = earth_radius_;
        }
        
        double g_magnitude = g0_ * (earth_radius_ / r) * (earth_radius_ / r);
        Vector3D gravity = position.normalized() * (-g_magnitude);
        
        // コリオリ力: -2 * Ω × v
        Vector3D omega(0.0, 0.0, rotation_rate_);  // 地球自転ベクトル（Z軸）
        Vector3D coriolis = omega.cross(velocity) * (-2.0);
        
        // 遠心力: -Ω × (Ω × r)
        Vector3D omega_cross_r = omega.cross(position);
        Vector3D centrifugal = omega.cross(omega_cross_r) * (-1.0);
        
        return gravity + coriolis + centrifugal;
    }
    
    int get_level() const override { return 3; }
    
    std::string get_description() const override {
        return "Rotating Earth with Coriolis effects (Level 3)";
    }
    
    /**
     * @brief パラメータを取得
     */
    double get_g0() const { return g0_; }
    double get_earth_radius() const { return earth_radius_; }
    double get_rotation_rate() const { return rotation_rate_; }
    
    /**
     * @brief パラメータを設定
     */
    void set_g0(double g0) { g0_ = g0; }
    void set_earth_radius(double radius) { earth_radius_ = radius; }
    void set_rotation_rate(double rate) { rotation_rate_ = rate; }
};

} // namespace ignis
