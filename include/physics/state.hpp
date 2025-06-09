#ifndef IGNISYEET_PHYSICS_STATE_HPP
#define IGNISYEET_PHYSICS_STATE_HPP

#include "vector3d.hpp"
#include "quaternion.hpp"
#include <ostream>

namespace IgnisYeet::Physics {

/**
 * @brief 3x3 Matrix for inertia tensors and transformations
 */
class Matrix3x3 {
private:
    double data_[9]; // Row-major storage

public:
    // Constructors
    Matrix3x3() {
        for (int i = 0; i < 9; ++i) data_[i] = 0.0;
    }
    
    Matrix3x3(double m00, double m01, double m02,
              double m10, double m11, double m12,
              double m20, double m21, double m22) {
        data_[0] = m00; data_[1] = m01; data_[2] = m02;
        data_[3] = m10; data_[4] = m11; data_[5] = m12;
        data_[6] = m20; data_[7] = m21; data_[8] = m22;
    }
    
    // Element access
    double& operator()(int row, int col) { return data_[row * 3 + col]; }
    const double& operator()(int row, int col) const { return data_[row * 3 + col]; }
    
    // Static constructors
    static Matrix3x3 identity() {
        return Matrix3x3(1.0, 0.0, 0.0,
                         0.0, 1.0, 0.0,
                         0.0, 0.0, 1.0);
    }
    
    static Matrix3x3 diagonal(double d0, double d1, double d2) {
        return Matrix3x3(d0,  0.0, 0.0,
                         0.0, d1,  0.0,
                         0.0, 0.0, d2);
    }
    
    // Matrix operations
    Vector3D operator*(const Vector3D& v) const {
        return Vector3D(
            data_[0] * v.x() + data_[1] * v.y() + data_[2] * v.z(),
            data_[3] * v.x() + data_[4] * v.y() + data_[5] * v.z(),
            data_[6] * v.x() + data_[7] * v.y() + data_[8] * v.z()
        );
    }
    
    Matrix3x3 inverse() const {
        // Simplified inverse for symmetric matrices (typical for inertia tensors)
        double det = data_[0] * (data_[4] * data_[8] - data_[5] * data_[7]) -
                     data_[1] * (data_[3] * data_[8] - data_[5] * data_[6]) +
                     data_[2] * (data_[3] * data_[7] - data_[4] * data_[6]);
        
        if (std::abs(det) < 1e-12) {
            return identity(); // Return identity if singular
        }
        
        Matrix3x3 inv;
        inv.data_[0] = (data_[4] * data_[8] - data_[5] * data_[7]) / det;
        inv.data_[1] = (data_[2] * data_[7] - data_[1] * data_[8]) / det;
        inv.data_[2] = (data_[1] * data_[5] - data_[2] * data_[4]) / det;
        inv.data_[3] = (data_[5] * data_[6] - data_[3] * data_[8]) / det;
        inv.data_[4] = (data_[0] * data_[8] - data_[2] * data_[6]) / det;
        inv.data_[5] = (data_[2] * data_[3] - data_[0] * data_[5]) / det;
        inv.data_[6] = (data_[3] * data_[7] - data_[4] * data_[6]) / det;
        inv.data_[7] = (data_[1] * data_[6] - data_[0] * data_[7]) / det;
        inv.data_[8] = (data_[0] * data_[4] - data_[1] * data_[3]) / det;
        
        return inv;
    }
};

/**
 * @brief Combined forces and moments acting on a rigid body
 */
struct ForcesMoments {
    Vector3D force;    // Total force [N]
    Vector3D moment;   // Total moment [N⋅m]
    
    ForcesMoments() 
        : force(Vector3D::zero())
        , moment(Vector3D::zero()) 
    {}
    
    ForcesMoments(const Vector3D& f, const Vector3D& m)
        : force(f), moment(m) 
    {}
    
    ForcesMoments& operator+=(const ForcesMoments& other) {
        force += other.force;
        moment += other.moment;
        return *this;
    }
};

/**
 * @brief Complete state of a rigid body in 6DOF
 */
struct RigidBodyState {
    // Translational state
    Vector3D position;       // Position [m]
    Vector3D velocity;       // Velocity [m/s]
    Vector3D acceleration;   // Acceleration [m/s²]
    
    // Rotational state
    Quaternion orientation;  // Orientation quaternion
    Vector3D angular_velocity;      // Angular velocity [rad/s]
    Vector3D angular_acceleration; // Angular acceleration [rad/s²]
    
    // Mass properties
    double mass;            // Current mass [kg]
    Matrix3x3 inertia_tensor;         // Inertia tensor [kg⋅m²]
    Matrix3x3 inertia_tensor_inverse; // Inverse inertia tensor [kg⋅m²]⁻¹
    
    // Time
    double time;            // Current simulation time [s]
    
    // Constructors
    RigidBodyState() 
        : position(Vector3D::zero())
        , velocity(Vector3D::zero())
        , acceleration(Vector3D::zero())
        , orientation(Quaternion::identity())
        , angular_velocity(Vector3D::zero())
        , angular_acceleration(Vector3D::zero())
        , mass(0.0)
        , inertia_tensor(Matrix3x3::identity())
        , inertia_tensor_inverse(Matrix3x3::identity())
        , time(0.0)
    {}
    
    RigidBodyState(const Vector3D& pos, const Vector3D& vel, 
                   const Quaternion& orient, double m, double t)
        : position(pos)
        , velocity(vel)
        , acceleration(Vector3D::zero())
        , orientation(orient)
        , angular_velocity(Vector3D::zero())
        , angular_acceleration(Vector3D::zero())
        , mass(m)
        , inertia_tensor(Matrix3x3::identity())
        , inertia_tensor_inverse(Matrix3x3::identity())
        , time(t)
    {}
    
    // Get kinetic energy
    double kinetic_energy() const {
        return 0.5 * mass * velocity.magnitude_squared();
    }
    
    // Get speed
    double speed() const {
        return velocity.magnitude();
    }
    
    // Get altitude (assuming z is up)
    double altitude() const {
        return position.z();
    }
    
    // Copy constructor and assignment
    RigidBodyState(const RigidBodyState&) = default;
    RigidBodyState& operator=(const RigidBodyState&) = default;
    RigidBodyState(RigidBodyState&&) = default;
    RigidBodyState& operator=(RigidBodyState&&) = default;
};

/**
 * @brief Environmental state at a point in space and time
 */
struct EnvironmentState {
    // Atmospheric properties
    double air_density;      // Air density [kg/m³]
    double temperature;      // Temperature [K]
    double pressure;         // Pressure [Pa]
    double sound_speed;      // Speed of sound [m/s]
    
    // Wind conditions
    Vector3D wind_velocity;  // Wind velocity [m/s]
    
    // Gravitational field
    Vector3D gravity;        // Gravitational acceleration [m/s²]
    
    // Default constructor (standard atmosphere at sea level)
    EnvironmentState()
        : air_density(1.225)
        , temperature(288.15)
        , pressure(101325.0)
        , sound_speed(343.0)
        , wind_velocity(Vector3D::zero())
        , gravity(Vector3D(0.0, 0.0, -9.80665))
    {}
    
    // Get Mach number for given velocity
    double mach_number(const Vector3D& velocity) const {
        return velocity.magnitude() / sound_speed;
    }
    
    // Get dynamic pressure
    double dynamic_pressure(const Vector3D& velocity) const {
        return 0.5 * air_density * velocity.magnitude_squared();
    }
};

// Stream output operators
inline std::ostream& operator<<(std::ostream& os, const RigidBodyState& state) {
    return os << "State{t=" << state.time 
              << ", pos=" << state.position
              << ", vel=" << state.velocity
              << ", mass=" << state.mass << "}";
}

inline std::ostream& operator<<(std::ostream& os, const EnvironmentState& env) {
    return os << "Environment{rho=" << env.air_density
              << ", T=" << env.temperature
              << ", wind=" << env.wind_velocity << "}";
}

inline std::ostream& operator<<(std::ostream& os, const ForcesMoments& fm) {
    return os << "ForcesMoments{F=" << fm.force << ", M=" << fm.moment << "}";
}

} // namespace IgnisYeet::Physics

#endif // IGNISYEET_PHYSICS_STATE_HPP
