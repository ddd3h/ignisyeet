#ifndef IGNISYEET_PHYSICS_QUATERNION_HPP
#define IGNISYEET_PHYSICS_QUATERNION_HPP

#include "vector3d.hpp"
#include <array>
#include <cmath>
#include <ostream>

namespace IgnisYeet::Physics {

/**
 * @brief Quaternion class for 3D rotations
 * Quaternion is stored as [w, x, y, z] where w is the scalar part
 */
class Quaternion {
private:
    std::array<double, 4> data_; // [w, x, y, z]

public:
    // Constructors
    Quaternion() : data_{1.0, 0.0, 0.0, 0.0} {}  // Identity quaternion
    Quaternion(double w, double x, double y, double z) : data_{w, x, y, z} {}
    Quaternion(const std::array<double, 4>& arr) : data_(arr) {}
    
    // Element access
    double& w() { return data_[0]; }
    const double& w() const { return data_[0]; }
    double& x() { return data_[1]; }
    const double& x() const { return data_[1]; }
    double& y() { return data_[2]; }
    const double& y() const { return data_[2]; }
    double& z() { return data_[3]; }
    const double& z() const { return data_[3]; }
    
    double& operator[](size_t idx) { return data_[idx]; }
    const double& operator[](size_t idx) const { return data_[idx]; }
    
    // Quaternion operations
    Quaternion operator*(const Quaternion& other) const {
        return Quaternion(
            w() * other.w() - x() * other.x() - y() * other.y() - z() * other.z(),
            w() * other.x() + x() * other.w() + y() * other.z() - z() * other.y(),
            w() * other.y() - x() * other.z() + y() * other.w() + z() * other.x(),
            w() * other.z() + x() * other.y() - y() * other.x() + z() * other.w()
        );
    }
    
    Quaternion& operator*=(const Quaternion& other) {
        *this = *this * other;
        return *this;
    }
    
    Quaternion operator+(const Quaternion& other) const {
        return Quaternion(w() + other.w(), x() + other.x(), 
                         y() + other.y(), z() + other.z());
    }
    
    Quaternion operator-(const Quaternion& other) const {
        return Quaternion(w() - other.w(), x() - other.x(), 
                         y() - other.y(), z() - other.z());
    }
    
    Quaternion operator*(double scalar) const {
        return Quaternion(w() * scalar, x() * scalar, 
                         y() * scalar, z() * scalar);
    }
    
    // Quaternion properties
    double magnitude() const {
        return std::sqrt(w() * w() + x() * x() + y() * y() + z() * z());
    }
    
    double magnitude_squared() const {
        return w() * w() + x() * x() + y() * y() + z() * z();
    }
    
    Quaternion normalized() const {
        double mag = magnitude();
        if (mag == 0.0) return Quaternion();
        return *this * (1.0 / mag);
    }
    
    void normalize() {
        double mag = magnitude();
        if (mag != 0.0) {
            data_[0] /= mag;
            data_[1] /= mag;
            data_[2] /= mag;
            data_[3] /= mag;
        }
    }
    
    Quaternion conjugate() const {
        return Quaternion(w(), -x(), -y(), -z());
    }
    
    Quaternion inverse() const {
        double mag_sq = magnitude_squared();
        if (mag_sq == 0.0) return Quaternion();
        return conjugate() * (1.0 / mag_sq);
    }
    
    // Rotation operations
    Vector3D rotate_vector(const Vector3D& v) const {
        // v' = q * v * q^(-1)
        Quaternion qv(0.0, v.x(), v.y(), v.z());
        Quaternion result = (*this) * qv * conjugate();
        return Vector3D(result.x(), result.y(), result.z());
    }
    
    // Alias for rotate_vector for convenience
    Vector3D rotate(const Vector3D& v) const {
        return rotate_vector(v);
    }
    
    // Conversion to/from Euler angles (ZYX convention)
    Vector3D to_euler() const {
        double roll = std::atan2(2.0 * (w() * x() + y() * z()),
                                1.0 - 2.0 * (x() * x() + y() * y()));
        double pitch = std::asin(2.0 * (w() * y() - z() * x()));
        double yaw = std::atan2(2.0 * (w() * z() + x() * y()),
                               1.0 - 2.0 * (y() * y() + z() * z()));
        return Vector3D(roll, pitch, yaw);
    }
    
    static Quaternion from_euler(const Vector3D& euler) {
        double cr = std::cos(euler.x() * 0.5);
        double sr = std::sin(euler.x() * 0.5);
        double cp = std::cos(euler.y() * 0.5);
        double sp = std::sin(euler.y() * 0.5);
        double cy = std::cos(euler.z() * 0.5);
        double sy = std::sin(euler.z() * 0.5);
        
        return Quaternion(
            cr * cp * cy + sr * sp * sy,
            sr * cp * cy - cr * sp * sy,
            cr * sp * cy + sr * cp * sy,
            cr * cp * sy - sr * sp * cy
        );
    }
    
    // Conversion to/from axis-angle
    static Quaternion from_axis_angle(const Vector3D& axis, double angle) {
        double half_angle = angle * 0.5;
        double sin_half = std::sin(half_angle);
        Vector3D normalized_axis = axis.normalized();
        
        return Quaternion(
            std::cos(half_angle),
            normalized_axis.x() * sin_half,
            normalized_axis.y() * sin_half,
            normalized_axis.z() * sin_half
        );
    }
    
    // Create quaternion from angular velocity and time step
    static Quaternion from_angular_velocity(const Vector3D& omega, double dt) {
        double omega_magnitude = omega.magnitude();
        if (omega_magnitude < 1e-10) {
            return Quaternion::identity();
        }
        
        double angle = omega_magnitude * dt;
        Vector3D axis = omega / omega_magnitude;
        return from_axis_angle(axis, angle);
    }
    
    void to_axis_angle(Vector3D& axis, double& angle) const {
        Quaternion q = normalized();
        angle = 2.0 * std::acos(q.w());
        double sin_half = std::sin(angle * 0.5);
        
        if (std::abs(sin_half) > 1e-10) {
            axis = Vector3D(q.x() / sin_half, q.y() / sin_half, q.z() / sin_half);
        } else {
            axis = Vector3D::unit_z();
        }
    }
    
    // Interpolation
    static Quaternion slerp(const Quaternion& q1, const Quaternion& q2, double t) {
        Quaternion qn1 = q1.normalized();
        Quaternion qn2 = q2.normalized();
        
        double dot = qn1.w() * qn2.w() + qn1.x() * qn2.x() + 
                    qn1.y() * qn2.y() + qn1.z() * qn2.z();
        
        // If dot product is negative, negate one quaternion to take shorter path
        if (dot < 0.0) {
            qn2 = qn2 * (-1.0);
            dot = -dot;
        }
        
        if (dot > 0.9995) {
            // Linear interpolation for very close quaternions
            return (qn1 * (1.0 - t) + qn2 * t).normalized();
        }
        
        double theta_0 = std::acos(dot);
        double theta = theta_0 * t;
        double sin_theta = std::sin(theta);
        double sin_theta_0 = std::sin(theta_0);
        
        double s0 = std::cos(theta) - dot * sin_theta / sin_theta_0;
        double s1 = sin_theta / sin_theta_0;
        
        return qn1 * s0 + qn2 * s1;
    }
    
    // Comparison
    bool operator==(const Quaternion& other) const {
        constexpr double epsilon = 1e-10;
        return std::abs(w() - other.w()) < epsilon &&
               std::abs(x() - other.x()) < epsilon &&
               std::abs(y() - other.y()) < epsilon &&
               std::abs(z() - other.z()) < epsilon;
    }
    
    bool operator!=(const Quaternion& other) const {
        return !(*this == other);
    }
    
    // Conversion
    const std::array<double, 4>& to_array() const { return data_; }
    
    // Static constructors
    static Quaternion identity() { return Quaternion(1.0, 0.0, 0.0, 0.0); }
};

// Stream output
inline std::ostream& operator<<(std::ostream& os, const Quaternion& q) {
    return os << "[" << q.w() << ", " << q.x() << ", " << q.y() << ", " << q.z() << "]";
}

// Scalar multiplication (scalar * quaternion)
inline Quaternion operator*(double scalar, const Quaternion& q) {
    return q * scalar;
}

} // namespace IgnisYeet::Physics

#endif // IGNISYEET_PHYSICS_QUATERNION_HPP
