#pragma once

#include "vector3d.hpp"
#include "state.hpp"
#include "atmosphere.hpp"
#include "../parameter.hpp"
#include <cmath>

namespace ignis {
namespace physics {

/**
 * @brief 空力学計算レベル定義
 */
enum class AerodynamicsLevel {
    LEVEL_1 = 1,  ///< 基本抗力のみ（定係数）
    LEVEL_2 = 2,  ///< Mach数依存抗力・基本揚力
    LEVEL_3 = 3   ///< 完全空力モデル（揚力・横力・圧縮性補正）
};

/**
 * @brief 空力係数構造体
 */
struct AeroCoefficients {
    double cd;    ///< 抗力係数
    double cl;    ///< 揚力係数  
    double cs;    ///< 横力係数
    double cm;    ///< ピッチングモーメント係数
    double cn;    ///< ヨーイングモーメント係数
    double cr;    ///< ローリングモーメント係数
};

/**
 * @brief ロケット空力形状パラメータ
 */
struct AeroGeometry {
    double reference_area;      ///< 基準面積 [m²]
    double reference_length;    ///< 基準長 [m]
    double center_of_pressure;  ///< 圧力中心位置 [m]
    double nose_cone_angle;     ///< ノーズコーン角度 [rad]
    double fin_area;           ///< フィン面積 [m²]
    double fin_aspect_ratio;   ///< フィンアスペクト比
};

/**
 * @brief 3段階空力学モデルクラス
 * 
 * Level 1: 基本抗力のみ（定係数）
 * Level 2: Mach数依存抗力・基本揚力
 * Level 3: 完全空力モデル（揚力・横力・圧縮性補正）
 */
class AerodynamicsModel {
private:
    AerodynamicsLevel level_;
    AeroGeometry geometry_;
    
    // Level 1: 基本係数
    double cd0_;        ///< 零攻角抗力係数
    
    // Level 2: Mach数依存パラメータ
    double cd_mach_transition_;  ///< Mach数遷移点
    double cd_supersonic_slope_; ///< 超音速抗力増加率
    double cl_alpha_;           ///< 揚力傾斜 [/rad]
    
    // Level 3: 高次パラメータ
    double cd_compressibility_; ///< 圧縮性補正係数
    double cl_mach_factor_;     ///< Mach数揚力補正
    double cs_beta_;            ///< 横力係数 [/rad]
    double reynolds_factor_;    ///< Reynolds数補正

public:
    /**
     * @brief コンストラクタ
     * @param level 空力計算レベル
     * @param params パラメータオブジェクト
     */
    AerodynamicsModel(AerodynamicsLevel level, const Parameter& params);
    
    /**
     * @brief 空力力・モーメント計算
     * @param state ロケット状態
     * @param atmo_state 大気状態
     * @param force_out 出力：空力力 [N]
     * @param moment_out 出力：空力モーメント [N⋅m]
     */
    void computeAerodynamics(const IgnisYeet::Physics::RigidBodyState& state, 
                           const AtmosphereModel::AtmosphereState& atmo_state,
                           Vector3D& force_out, 
                           Vector3D& moment_out) const;
    
    /**
     * @brief 空力係数計算
     * @param mach_number Mach数
     * @param reynolds_number Reynolds数
     * @param angle_of_attack 攻角 [rad]
     * @param sideslip_angle 横滑り角 [rad]
     * @return 空力係数
     */
    AeroCoefficients computeCoefficients(double mach_number,
                                       double reynolds_number,
                                       double angle_of_attack,
                                       double sideslip_angle) const;

private:
    // Level別計算メソッド
    void computeLevel1(const IgnisYeet::Physics::RigidBodyState& state, const AtmosphereModel::AtmosphereState& atmo_state,
                      Vector3D& force, Vector3D& moment) const;
    void computeLevel2(const IgnisYeet::Physics::RigidBodyState& state, const AtmosphereModel::AtmosphereState& atmo_state,
                      Vector3D& force, Vector3D& moment) const;
    void computeLevel3(const IgnisYeet::Physics::RigidBodyState& state, const AtmosphereModel::AtmosphereState& atmo_state,
                      Vector3D& force, Vector3D& moment) const;
    
    // 補助計算関数
    double computeMachNumber(const Vector3D& velocity, double sound_speed) const;
    double computeReynoldsNumber(const Vector3D& velocity, double density, 
                               double viscosity, double length) const;
    std::pair<double, double> computeFlowAngles(const Vector3D& velocity_body) const;
    double computePrandtlGlauertCorrection(double mach_number) const;
    double computeSupersonicCdCorrection(double mach_number) const;
    
    // 係数補間・補正関数
    double interpolateMachEffect(double mach, double subsonic_value, 
                               double supersonic_value) const;
    double applyReynoldsCorrection(double base_coefficient, double reynolds) const;
    
    // 座標変換
    Vector3D transformToBodyFrame(const Vector3D& velocity_eci, 
                                const IgnisYeet::Physics::Quaternion& attitude) const;
    Vector3D transformToEciFrame(const Vector3D& force_body, 
                               const IgnisYeet::Physics::Quaternion& attitude) const;
};

// 定数定義
namespace aero_constants {
    constexpr double SOUND_SPEED_SEA_LEVEL = 343.0;  ///< 海面音速 [m/s]
    constexpr double KINEMATIC_VISCOSITY_SEA_LEVEL = 1.46e-5; ///< 海面動粘度 [m²/s]
    constexpr double CRITICAL_MACH = 0.8;            ///< 臨界Mach数
    constexpr double SUPERSONIC_MACH = 1.2;          ///< 超音速遷移Mach数
}

} // namespace physics
} // namespace ignis
