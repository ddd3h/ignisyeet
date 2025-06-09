#ifndef IGNISYEET_TYPES_HPP
#define IGNISYEET_TYPES_HPP

#include <array>
#include <vector>
#include <string>

namespace IgnisYeet {

// ============================================================================
// BASIC MATHEMATICAL TYPES
// ============================================================================

using Vector3 = std::array<double, 3>;
using Vector4 = std::array<double, 4>;
using Matrix3x3 = std::array<std::array<double, 3>, 3>;

// ============================================================================
// COORDINATE SYSTEMS
// ============================================================================

enum class CoordinateSystem {
    ECEF,        // Earth-Centered, Earth-Fixed
    ENU,         // East-North-Up (local)
    LATLON,      // Latitude-Longitude-Altitude
    BODY         // Body-fixed coordinates
};

// ============================================================================
// INTEGRATION METHODS
// ============================================================================

enum class IntegrationMethod {
    EULER,
    LEAPFROG,
    RK4,
    RK45,
    ADAPTIVE_RK45
};

// ============================================================================
// SIMULATION STATE
// ============================================================================

struct SimulationState {
    double time;
    Vector3 position;           // Position [m]
    Vector3 velocity;           // Velocity [m/s]
    Vector3 acceleration;       // Acceleration [m/s²]
    Vector3 angular_velocity;   // Angular velocity [rad/s]
    Vector3 angular_acceleration; // Angular acceleration [rad/s²]
    Vector4 quaternion;         // Orientation quaternion [w, x, y, z]
    double mass;                // Current mass [kg]
    double thrust;              // Current thrust [N]
    
    // Environmental state
    Vector3 wind_velocity;      // Wind velocity [m/s]
    double air_density;         // Air density [kg/m³]
    double temperature;         // Temperature [K]
    double pressure;           // Pressure [Pa]
    
    // Forces and moments
    Vector3 forces;            // Total forces [N]
    Vector3 moments;           // Total moments [N⋅m]
    
    SimulationState();
};

// ============================================================================
// PHYSICAL CONSTANTS
// ============================================================================

namespace Constants {
    constexpr double EARTH_RADIUS = 6371000.0;       // [m]
    constexpr double STANDARD_GRAVITY = 9.80665;     // [m/s²]
    constexpr double STANDARD_PRESSURE = 101325.0;   // [Pa]
    constexpr double STANDARD_TEMPERATURE = 288.15;  // [K]
    constexpr double STANDARD_DENSITY = 1.225;       // [kg/m³]
    constexpr double GAS_CONSTANT_DRY_AIR = 287.0;   // [J/(kg⋅K)]
    constexpr double EARTH_ROTATION_RATE = 7.2921159e-5; // [rad/s]
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

// Vector operations
Vector3 cross_product(const Vector3& a, const Vector3& b);
double dot_product(const Vector3& a, const Vector3& b);
double magnitude(const Vector3& v);
Vector3 normalize(const Vector3& v);
Vector3 add(const Vector3& a, const Vector3& b);
Vector3 subtract(const Vector3& a, const Vector3& b);
Vector3 multiply(const Vector3& v, double scalar);

// Quaternion operations
Vector4 quaternion_multiply(const Vector4& q1, const Vector4& q2);
Vector4 quaternion_normalize(const Vector4& q);
Vector3 quaternion_to_euler(const Vector4& q);
Vector4 euler_to_quaternion(const Vector3& euler);

// Coordinate transformations
Vector3 ecef_to_enu(const Vector3& ecef, const Vector3& ref_lla);
Vector3 enu_to_ecef(const Vector3& enu, const Vector3& ref_lla);
Vector3 ecef_to_lla(const Vector3& ecef);
Vector3 lla_to_ecef(const Vector3& lla);

} // namespace IgnisYeet

#endif // IGNISYEET_TYPES_HPP
