#ifndef IGNISYEET_PHYSICS_VECTOR3D_HPP
#define IGNISYEET_PHYSICS_VECTOR3D_HPP

#include <array>
#include <cmath>
#include <ostream>

namespace IgnisYeet::Physics {

/**
 * @brief 3D vector class with common vector operations
 */
class Vector3D {
private:
    std::array<double, 3> data_;

public:
    // Constructors
    Vector3D() : data_{0.0, 0.0, 0.0} {}
    Vector3D(double x, double y, double z) : data_{x, y, z} {}
    Vector3D(const std::array<double, 3>& arr) : data_(arr) {}
    
    // Element access
    double& operator[](size_t idx) { return data_[idx]; }
    const double& operator[](size_t idx) const { return data_[idx]; }
    
    double& x() { return data_[0]; }
    const double& x() const { return data_[0]; }
    double& y() { return data_[1]; }
    const double& y() const { return data_[1]; }
    double& z() { return data_[2]; }
    const double& z() const { return data_[2]; }
    
    // Arithmetic operations
    Vector3D operator+(const Vector3D& other) const {
        return Vector3D(data_[0] + other.data_[0], 
                       data_[1] + other.data_[1], 
                       data_[2] + other.data_[2]);
    }
    
    Vector3D operator-(const Vector3D& other) const {
        return Vector3D(data_[0] - other.data_[0], 
                       data_[1] - other.data_[1], 
                       data_[2] - other.data_[2]);
    }
    
    // Unary minus operator  
    Vector3D operator-() const {
        return Vector3D(-data_[0], -data_[1], -data_[2]);
    }
    
    Vector3D operator*(double scalar) const {
        return Vector3D(data_[0] * scalar, 
                       data_[1] * scalar, 
                       data_[2] * scalar);
    }
    
    Vector3D operator/(double scalar) const {
        return Vector3D(data_[0] / scalar, 
                       data_[1] / scalar, 
                       data_[2] / scalar);
    }
    
    Vector3D& operator+=(const Vector3D& other) {
        data_[0] += other.data_[0];
        data_[1] += other.data_[1];
        data_[2] += other.data_[2];
        return *this;
    }
    
    Vector3D& operator-=(const Vector3D& other) {
        data_[0] -= other.data_[0];
        data_[1] -= other.data_[1];
        data_[2] -= other.data_[2];
        return *this;
    }
    
    Vector3D& operator*=(double scalar) {
        data_[0] *= scalar;
        data_[1] *= scalar;
        data_[2] *= scalar;
        return *this;
    }
    
    Vector3D& operator/=(double scalar) {
        data_[0] /= scalar;
        data_[1] /= scalar;
        data_[2] /= scalar;
        return *this;
    }
    
    // Vector operations
    double dot(const Vector3D& other) const {
        return data_[0] * other.data_[0] + 
               data_[1] * other.data_[1] + 
               data_[2] * other.data_[2];
    }
    
    Vector3D cross(const Vector3D& other) const {
        return Vector3D(
            data_[1] * other.data_[2] - data_[2] * other.data_[1],
            data_[2] * other.data_[0] - data_[0] * other.data_[2],
            data_[0] * other.data_[1] - data_[1] * other.data_[0]
        );
    }
    
    double magnitude() const {
        return std::sqrt(data_[0] * data_[0] + 
                        data_[1] * data_[1] + 
                        data_[2] * data_[2]);
    }
    
    double magnitude_squared() const {
        return data_[0] * data_[0] + 
               data_[1] * data_[1] + 
               data_[2] * data_[2];
    }
    
    Vector3D normalized() const {
        double mag = magnitude();
        if (mag == 0.0) return Vector3D();
        return *this / mag;
    }
    
    void normalize() {
        double mag = magnitude();
        if (mag != 0.0) {
            *this /= mag;
        }
    }
    
    // Comparison
    bool operator==(const Vector3D& other) const {
        constexpr double epsilon = 1e-10;
        return std::abs(data_[0] - other.data_[0]) < epsilon &&
               std::abs(data_[1] - other.data_[1]) < epsilon &&
               std::abs(data_[2] - other.data_[2]) < epsilon;
    }
    
    bool operator!=(const Vector3D& other) const {
        return !(*this == other);
    }
    
    // Conversion
    const std::array<double, 3>& to_array() const { return data_; }
    
    // Zero vector
    static Vector3D zero() { return Vector3D(0.0, 0.0, 0.0); }
    
    // Unit vectors
    static Vector3D unit_x() { return Vector3D(1.0, 0.0, 0.0); }
    static Vector3D unit_y() { return Vector3D(0.0, 1.0, 0.0); }
    static Vector3D unit_z() { return Vector3D(0.0, 0.0, 1.0); }
};

// Stream output
inline std::ostream& operator<<(std::ostream& os, const Vector3D& v) {
    return os << "[" << v[0] << ", " << v[1] << ", " << v[2] << "]";
}

// Scalar multiplication (scalar * vector)
inline Vector3D operator*(double scalar, const Vector3D& v) {
    return v * scalar;
}

} // namespace IgnisYeet::Physics

#endif // IGNISYEET_PHYSICS_VECTOR3D_HPP
