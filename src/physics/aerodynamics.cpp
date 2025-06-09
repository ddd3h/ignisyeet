#include "../include/physics/aerodynamics.hpp"
#include <algorithm>
#include <stdexcept>

namespace ignis {
namespace physics {

AerodynamicsModel::AerodynamicsModel(AerodynamicsLevel level, const Parameter& params) 
    : level_(level) {
    
    // 基本形状パラメータの読み込み
    geometry_.reference_area = params.aerodynamics.reference_area;
    geometry_.reference_length = params.aerodynamics.reference_length; 
    geometry_.center_of_pressure = params.aerodynamics.center_of_pressure;
    geometry_.nose_cone_angle = params.aerodynamics.nose_cone_angle;
    geometry_.fin_area = params.aerodynamics.fin_area;
    // Note: fin_aspect_ratio not in current AerodynamicsConfig, using default
    geometry_.fin_aspect_ratio = 2.0; // Default value
    
    // Level 1: 基本係数
    cd0_ = params.aerodynamics.cd_constant; // Using cd_constant as cd0
    
    // Level 2: Mach数依存パラメータ
    if (level_ >= AerodynamicsLevel::LEVEL_2) {
        cd_mach_transition_ = params.aerodynamics.mach_critical; // Using mach_critical
        cd_supersonic_slope_ = params.aerodynamics.mach_drag_rise; // Using mach_drag_rise
        cl_alpha_ = params.aerodynamics.cl_alpha;
    }
    
    // Level 3: 高次パラメータ
    if (level_ >= AerodynamicsLevel::LEVEL_3) {
        // Using default values for missing parameters in current AerodynamicsConfig
        cd_compressibility_ = 0.1;
        cl_mach_factor_ = 0.8;
        cs_beta_ = params.aerodynamics.cy_beta; // Using cy_beta as cs_beta
        reynolds_factor_ = 0.05;
    }
}

void AerodynamicsModel::computeAerodynamics(const IgnisYeet::Physics::RigidBodyState& state, 
                                          const AtmosphereModel::AtmosphereState& atmo_state,
                                          Vector3D& force_out, 
                                          Vector3D& moment_out) const {
    switch (level_) {
        case AerodynamicsLevel::LEVEL_1:
            computeLevel1(state, atmo_state, force_out, moment_out);
            break;
        case AerodynamicsLevel::LEVEL_2:
            computeLevel2(state, atmo_state, force_out, moment_out);
            break;
        case AerodynamicsLevel::LEVEL_3:
            computeLevel3(state, atmo_state, force_out, moment_out);
            break;
        default:
            throw std::invalid_argument("Invalid aerodynamics level");
    }
}

void AerodynamicsModel::computeLevel1(const IgnisYeet::Physics::RigidBodyState& state, const AtmosphereModel::AtmosphereState& atmo_state,
                                    Vector3D& force, Vector3D& moment) const {
    // Level 1: 基本抗力のみ（速度方向反対）
    const Vector3D& velocity = state.velocity;
    double velocity_magnitude = velocity.magnitude();
    
    if (velocity_magnitude < 1e-6) {
        force = Vector3D(0, 0, 0);
        moment = Vector3D(0, 0, 0);
        return;
    }
    
    // 動圧計算
    double dynamic_pressure = 0.5 * atmo_state.density * velocity_magnitude * velocity_magnitude;
    
    // 抗力計算（速度方向反対）
    double drag_magnitude = cd0_ * geometry_.reference_area * dynamic_pressure;
    Vector3D drag_direction = velocity.normalized() * (-1.0);
    force = drag_direction * drag_magnitude;
    
    // Level 1では空力モーメントなし
    moment = Vector3D(0, 0, 0);
}

void AerodynamicsModel::computeLevel2(const IgnisYeet::Physics::RigidBodyState& state, const AtmosphereModel::AtmosphereState& atmo_state,
                                    Vector3D& force, Vector3D& moment) const {
    // Level 2: Mach数依存抗力 + 基本揚力
    const Vector3D& velocity = state.velocity;
    double velocity_magnitude = velocity.magnitude();
    
    if (velocity_magnitude < 1e-6) {
        force = Vector3D(0, 0, 0);
        moment = Vector3D(0, 0, 0);
        return;
    }
    
    // 音速・Mach数計算
    double sound_speed = std::sqrt(1.4 * 287.0 * atmo_state.temperature);
    double mach_number = computeMachNumber(velocity, sound_speed);
    
    // 機体座標系での速度
    Vector3D velocity_body = transformToBodyFrame(velocity, state.orientation);
    auto [angle_of_attack, sideslip_angle] = computeFlowAngles(velocity_body);
    
    // 動圧
    double dynamic_pressure = 0.5 * atmo_state.density * velocity_magnitude * velocity_magnitude;
    
    // Mach数依存抗力係数
    double cd = cd0_;
    if (mach_number > cd_mach_transition_) {
        cd += cd_supersonic_slope_ * (mach_number - cd_mach_transition_);
    }
    
    // 揚力係数（攻角依存）
    double cl = cl_alpha_ * angle_of_attack;
    
    // 機体座標系での力
    Vector3D force_body;
    force_body.x() = -cd * geometry_.reference_area * dynamic_pressure;  // 抗力（x軸負方向）
    force_body.y() = 0.0;  // 横力なし
    force_body.z() = -cl * geometry_.reference_area * dynamic_pressure; // 揚力（z軸負方向、NED座標系）
    
    // ECI座標系に変換
    force = transformToEciFrame(force_body, state.orientation);
    
    // 基本ピッチングモーメント
    double cm = -cl * geometry_.center_of_pressure / geometry_.reference_length;
    moment = Vector3D(0, cm * geometry_.reference_area * geometry_.reference_length * dynamic_pressure, 0);
}

void AerodynamicsModel::computeLevel3(const IgnisYeet::Physics::RigidBodyState& state, const AtmosphereModel::AtmosphereState& atmo_state,
                                    Vector3D& force, Vector3D& moment) const {
    // Level 3: 完全空力モデル
    const Vector3D& velocity = state.velocity;
    double velocity_magnitude = velocity.magnitude();
    
    if (velocity_magnitude < 1e-6) {
        force = Vector3D(0, 0, 0);
        moment = Vector3D(0, 0, 0);
        return;
    }
    
    // 音速・Mach数・Reynolds数計算
    double sound_speed = std::sqrt(1.4 * 287.0 * atmo_state.temperature);
    double mach_number = computeMachNumber(velocity, sound_speed);
    double kinematic_viscosity = aero_constants::KINEMATIC_VISCOSITY_SEA_LEVEL * 
                                (atmo_state.temperature / 288.15) * 
                                std::pow(atmo_state.pressure / 101325.0, -0.5);
    double reynolds_number = computeReynoldsNumber(velocity, atmo_state.density, 
                                                 kinematic_viscosity, geometry_.reference_length);
    
    // 機体座標系での速度と流れ角
    Vector3D velocity_body = transformToBodyFrame(velocity, state.orientation);
    auto [angle_of_attack, sideslip_angle] = computeFlowAngles(velocity_body);
    
    // 空力係数計算
    AeroCoefficients coeffs = computeCoefficients(mach_number, reynolds_number, 
                                                angle_of_attack, sideslip_angle);
    
    // 動圧
    double dynamic_pressure = 0.5 * atmo_state.density * velocity_magnitude * velocity_magnitude;
    
    // 機体座標系での力とモーメント
    Vector3D force_body;
    force_body.x() = -coeffs.cd * geometry_.reference_area * dynamic_pressure;  // 抗力
    force_body.y() = coeffs.cs * geometry_.reference_area * dynamic_pressure;   // 横力
    force_body.z() = -coeffs.cl * geometry_.reference_area * dynamic_pressure;  // 揚力
    
    Vector3D moment_body;
    moment_body.x() = coeffs.cr * geometry_.reference_area * geometry_.reference_length * dynamic_pressure; // ロール
    moment_body.y() = coeffs.cm * geometry_.reference_area * geometry_.reference_length * dynamic_pressure; // ピッチ
    moment_body.z() = coeffs.cn * geometry_.reference_area * geometry_.reference_length * dynamic_pressure; // ヨー
    
    // ECI座標系に変換
    force = transformToEciFrame(force_body, state.orientation);
    moment = transformToEciFrame(moment_body, state.orientation);
}

AeroCoefficients AerodynamicsModel::computeCoefficients(double mach_number,
                                                      double reynolds_number,
                                                      double angle_of_attack,
                                                      double sideslip_angle) const {
    AeroCoefficients coeffs;
    
    // 抗力係数：Mach数・圧縮性・Reynolds数依存
    coeffs.cd = cd0_;
    
    // Mach数依存
    if (level_ >= AerodynamicsLevel::LEVEL_2) {
        if (mach_number > cd_mach_transition_) {
            coeffs.cd += cd_supersonic_slope_ * (mach_number - cd_mach_transition_);
        }
    }
    
    // 圧縮性補正（Level 3）
    if (level_ >= AerodynamicsLevel::LEVEL_3) {
        if (mach_number < aero_constants::CRITICAL_MACH) {
            coeffs.cd *= computePrandtlGlauertCorrection(mach_number);
        } else {
            coeffs.cd *= computeSupersonicCdCorrection(mach_number);
        }
        
        // Reynolds数補正
        coeffs.cd = applyReynoldsCorrection(coeffs.cd, reynolds_number);
    }
    
    // 揚力係数
    coeffs.cl = 0.0;
    if (level_ >= AerodynamicsLevel::LEVEL_2) {
        coeffs.cl = cl_alpha_ * angle_of_attack;
        
        // Level 3: Mach数補正
        if (level_ >= AerodynamicsLevel::LEVEL_3) {
            coeffs.cl *= cl_mach_factor_ * computePrandtlGlauertCorrection(mach_number);
        }
    }
    
    // 横力係数（Level 3のみ）
    coeffs.cs = 0.0;
    if (level_ >= AerodynamicsLevel::LEVEL_3) {
        coeffs.cs = cs_beta_ * sideslip_angle;
    }
    
    // モーメント係数
    coeffs.cm = (level_ >= AerodynamicsLevel::LEVEL_2) ? 
                -coeffs.cl * geometry_.center_of_pressure / geometry_.reference_length : 0.0;
    coeffs.cn = (level_ >= AerodynamicsLevel::LEVEL_3) ? 
                -coeffs.cs * geometry_.center_of_pressure / geometry_.reference_length : 0.0;
    coeffs.cr = 0.0; // ロールモーメント（簡易的に0）
    
    return coeffs;
}

// 補助計算関数の実装
double AerodynamicsModel::computeMachNumber(const Vector3D& velocity, double sound_speed) const {
    return velocity.magnitude() / sound_speed;
}

double AerodynamicsModel::computeReynoldsNumber(const Vector3D& velocity, double density, 
                                              double viscosity, double length) const {
    return velocity.magnitude() * length / viscosity;
}

std::pair<double, double> AerodynamicsModel::computeFlowAngles(const Vector3D& velocity_body) const {
    double velocity_magnitude = velocity_body.magnitude();
    if (velocity_magnitude < 1e-6) {
        return {0.0, 0.0};
    }
    
    // 攻角：x-z平面での角度
    double angle_of_attack = std::atan2(-velocity_body.z(), velocity_body.x());
    
    // 横滑り角：x-y平面での角度
    double sideslip_angle = std::atan2(velocity_body.y(), velocity_body.x());
    
    return {angle_of_attack, sideslip_angle};
}

double AerodynamicsModel::computePrandtlGlauertCorrection(double mach_number) const {
    if (mach_number >= 1.0) return 1.0;
    return 1.0 / std::sqrt(1.0 - mach_number * mach_number);
}

double AerodynamicsModel::computeSupersonicCdCorrection(double mach_number) const {
    if (mach_number < 1.0) return 1.0;
    
    // 簡易的な超音速抗力増加モデル
    double mach_factor = std::min(mach_number, 5.0); // Mach 5でキャップ
    return 1.0 + cd_compressibility_ * (mach_factor - 1.0);
}

double AerodynamicsModel::applyReynoldsCorrection(double base_coefficient, double reynolds) const {
    // 簡易Reynolds数補正（対数的減少）
    double reynolds_correction = 1.0 - reynolds_factor_ * std::log10(std::max(reynolds / 1e6, 0.1));
    return base_coefficient * std::max(reynolds_correction, 0.5);
}

Vector3D AerodynamicsModel::transformToBodyFrame(const Vector3D& velocity_eci, 
                                                const IgnisYeet::Physics::Quaternion& attitude) const {
    // クォータニオンによる座標変換（ECI → Body）
    return attitude.inverse().rotate_vector(velocity_eci);
}

Vector3D AerodynamicsModel::transformToEciFrame(const Vector3D& force_body, 
                                               const IgnisYeet::Physics::Quaternion& attitude) const {
    // クォータニオンによる座標変換（Body → ECI）
    return attitude.rotate_vector(force_body);
}

} // namespace physics
} // namespace ignis
