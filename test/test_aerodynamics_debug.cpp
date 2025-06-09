#include "../include/physics/aerodynamics.hpp"
#include "../include/physics/state.hpp"
#include "../include/physics/vector3d.hpp"
#include "../include/physics/quaternion.hpp"
#include "../include/physics/atmosphere.hpp"
#include "../include/parameter.hpp"
#include <iostream>
#include <iomanip>
#include <cmath>

using namespace IgnisYeet::Physics;
using ignis::physics::AerodynamicsModel;
using ignis::physics::AerodynamicsLevel;

int main() {
    std::cout << "🔍 IgnisYeet Aerodynamics Debug Test" << std::endl;
    std::cout << "====================================" << std::endl;
    
    try {
        // Load parameters and check values
        Parameter params("../parameter.toml");
        params.loadParameters();
        
        std::cout << "\n📊 Parameter Values:" << std::endl;
        std::cout << "Reference area: " << params.aerodynamics.reference_area << " m²" << std::endl;
        std::cout << "Reference length: " << params.aerodynamics.reference_length << " m" << std::endl;
        std::cout << "CD0: " << params.aerodynamics.cd_constant << std::endl;
        std::cout << "CL alpha: " << params.aerodynamics.cl_alpha << std::endl;
        std::cout << "Center of pressure: " << params.aerodynamics.center_of_pressure << " m" << std::endl;
        std::cout << "Nose cone angle: " << params.aerodynamics.nose_cone_angle << " rad" << std::endl;
        std::cout << "Fin area: " << params.aerodynamics.fin_area << " m²" << std::endl;
        
        // Initialize aerodynamics model
        AerodynamicsModel aero(AerodynamicsLevel::LEVEL_1, params);
        
        // Test state (velocity in x-direction)
        RigidBodyState state;
        state.position = Vector3D(0, 0, 1000);  // 1km altitude
        state.velocity = Vector3D(100, 0, 0);    // 100 m/s velocity
        state.orientation = Quaternion::identity(); // Identity quaternion
        
        // Atmosphere state
        ignis::AtmosphereModel::AtmosphereState atmo_state;
        atmo_state.density = 1.225;      // Sea level density
        atmo_state.temperature = 288.15; // Sea level temperature
        atmo_state.pressure = 101325;    // Sea level pressure
        
        std::cout << "\n🌬️ Atmosphere Conditions:" << std::endl;
        std::cout << "Density: " << atmo_state.density << " kg/m³" << std::endl;
        std::cout << "Temperature: " << atmo_state.temperature << " K" << std::endl;
        std::cout << "Pressure: " << atmo_state.pressure << " Pa" << std::endl;
        
        // Compute aerodynamics
        Vector3D force, moment;
        aero.computeAerodynamics(state, atmo_state, force, moment);
        
        std::cout << "\n📈 Results:" << std::endl;
        std::cout << "Velocity: (" << state.velocity.x() << ", " << state.velocity.y() << ", " << state.velocity.z() << ") m/s" << std::endl;
        std::cout << "Velocity magnitude: " << state.velocity.magnitude() << " m/s" << std::endl;
        std::cout << "Aero Force: (" << force.x() << ", " << force.y() << ", " << force.z() << ") N" << std::endl;
        std::cout << "Aero Moment: (" << moment.x() << ", " << moment.y() << ", " << moment.z() << ") N⋅m" << std::endl;
        
        // Expected calculations
        double dynamic_pressure = 0.5 * atmo_state.density * std::pow(state.velocity.magnitude(), 2);
        double expected_drag = params.aerodynamics.cd_constant * params.aerodynamics.reference_area * dynamic_pressure;
        
        std::cout << "\n🧮 Expected Calculations:" << std::endl;
        std::cout << "Dynamic pressure: " << dynamic_pressure << " Pa" << std::endl;
        std::cout << "Expected drag force: " << expected_drag << " N" << std::endl;
        
        std::cout << "\n✅ Debug test completed!" << std::endl;
        
    } catch (const std::exception& e) {
        std::cerr << "\n❌ Test failed with exception: " << e.what() << std::endl;
        return 1;
    }
    
    return 0;
}
