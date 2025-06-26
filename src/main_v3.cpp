#include <iostream>
#include <iomanip>
#include <thread>
#include <chrono>
#include <filesystem>
#include <memory>
#include <string>

// New architecture includes
#include "simulation/engine.hpp"
#include "rocket/vehicle.hpp"
#include "environment/atmosphere.hpp"
#include "output/manager.hpp"
#include "physics/vector3d.hpp"
#include "physics/quaternion.hpp"
#include "physics/state.hpp"
#include "parameter.hpp"

using namespace IgnisYeet;

void print_progress(const std::string& stage, double progress) {
    int bar_width = 50;
    int pos = static_cast<int>(progress * bar_width);

    std::cout << "\r" << stage << " [";
    for (int i = 0; i < bar_width; i++) {
        if (i < pos) std::cout << "=";
        else if (i == pos) std::cout << ">";
        else std::cout << " ";
    }
    std::cout << "] " << std::fixed << std::setprecision(1) << (progress * 100.0) << "%" << std::flush;
}

void print_header() {
    std::cout << "\n" << std::string(80, '=') << std::endl;
    std::cout << "🚀 IgnisYeet - 6DOF Rocket Simulation v3.0 (New Architecture)" << std::endl;
    std::cout << std::string(80, '=') << std::endl;
}

void print_configuration(const Parameter& param) {
    std::cout << "\n📋 Simulation Configuration:" << std::endl;
    std::cout << "  Name: " << param.simulation.name << std::endl;
    std::cout << "  Integration Method: " << param.simulation.integration_method << std::endl;
    std::cout << "  Time Step: " << param.simulation.time_step << " s" << std::endl;
    std::cout << "  Max Time: " << param.simulation.max_time << " s" << std::endl;
    std::cout << "  Physics Level: " << param.simulation.physics_level << std::endl;
    std::cout << "  Output Interval: " << param.simulation.output_interval << " s" << std::endl;
    
    std::cout << "\n🚀 Rocket Configuration:" << std::endl;
    std::cout << "  Dry Mass: " << param.rocket_config.mass_dry << " kg" << std::endl;
    std::cout << "  Propellant Mass: " << param.rocket_config.mass_propellant << " kg" << std::endl;
    std::cout << "  Total Mass: " << param.rocket_config.mass_total << " kg" << std::endl;
    std::cout << "  Length: " << param.rocket_config.length << " m" << std::endl;
    std::cout << "  Diameter: " << param.rocket_config.diameter << " m" << std::endl;
    
    std::cout << "\n🌍 Environment Configuration:" << std::endl;
    std::cout << "  Atmosphere Model: " << param.environment_config.atmosphere_model << std::endl;
    std::cout << "  Gravity Model: " << param.environment_config.gravity_model << std::endl;
    std::cout << "  Wind Model: " << param.environment_config.wind_model << std::endl;
    
    std::cout << "\n🎯 Launch Configuration:" << std::endl;
    std::cout << "  Latitude: " << param.launch.latitude << "°" << std::endl;
    std::cout << "  Longitude: " << param.launch.longitude << "°" << std::endl;
    std::cout << "  Altitude: " << param.launch.altitude << " m" << std::endl;
    std::cout << "  Azimuth: " << param.launch.azimuth << "°" << std::endl;
    std::cout << "  Elevation: " << param.launch.elevation << "°" << std::endl;
}

Simulation::EngineConfig create_engine_config(const Parameter& param) {
    Simulation::EngineConfig config;
    config.max_time = param.simulation.max_time;
    config.dt = param.simulation.time_step;
    config.integrator_type = param.simulation.integration_method;
    config.physics_level = param.simulation.physics_level;
    config.output_interval = param.simulation.output_interval;
    
    config.terminate_on_ground = param.simulation.terminate_on_ground;
    config.terminate_on_max_time = param.simulation.terminate_on_max_time;
    config.terminate_on_max_altitude = param.simulation.terminate_on_max_altitude;
    config.max_altitude_limit = param.simulation.max_altitude_limit;
    
    return config;
}

Rocket::VehicleConfig create_vehicle_config(const Parameter& param) {
    Rocket::VehicleConfig config;
    config.mass_dry = param.rocket_config.mass_dry;
    config.mass_propellant = param.rocket_config.mass_propellant;
    config.length = param.rocket_config.length;
    config.diameter = param.rocket_config.diameter;
    config.reference_area = M_PI * std::pow(param.rocket_config.diameter / 2.0, 2);
    
    // Aerodynamics
    config.drag_coefficient = param.aerodynamics.cd_constant;
    config.lift_coefficient = param.aerodynamics.cl_alpha;
    config.normal_force_coefficient = param.aerodynamics.cn_alpha;
    
    // Inertia (simplified calculation based on cylinder)
    double radius = param.rocket_config.diameter / 2.0;
    double total_mass = param.rocket_config.mass_total;
    config.inertia_xx = total_mass * (3 * radius * radius + param.rocket_config.length * param.rocket_config.length) / 12.0;
    config.inertia_yy = config.inertia_xx;
    config.inertia_zz = total_mass * radius * radius / 2.0;
    
    return config;
}

Rocket::PropulsionConfig create_propulsion_config(const Parameter& param) {
    Rocket::PropulsionConfig config;
    config.isp = param.propulsion.isp;
    config.burn_time = param.propulsion.burn_time;
    config.propellant_mass = param.rocket_config.mass_propellant;
    
    // Create simple thrust curve if not provided
    if (param.propulsion.thrust_curve.empty()) {
        double max_thrust = param.rocket_config.mass_total * 9.81 * 2.0; // 2g acceleration
        config.thrust_curve = {
            {0.0, max_thrust},
            {param.propulsion.burn_time, max_thrust},
            {param.propulsion.burn_time + 0.1, 0.0}
        };
    } else {
        config.thrust_curve = param.propulsion.thrust_curve;
    }
    
    return config;
}

AtmosphereConfig create_atmosphere_config(const Parameter& param) {
    AtmosphereConfig config;
    config.model_type = param.environment_config.atmosphere_model;
    config.sea_level_pressure = 101325.0;
    config.sea_level_temperature = 288.15;
    config.sea_level_density = 1.225;
    config.scale_height = 8400.0;
    config.temperature_lapse_rate = -0.0065;
    
    return config;
}

GravityConfig create_gravity_config(const Parameter& param) {
    GravityConfig config;
    config.model_type = param.environment_config.gravity_model;
    config.surface_gravity = 9.81;
    config.earth_radius = 6371000.0;
    
    return config;
}

WindConfig create_wind_config(const Parameter& param) {
    WindConfig config;
    config.model_type = param.environment_config.wind_model;
    config.ground_speed = param.environment_config.wind.ground_speed;
    config.ground_direction = param.environment_config.wind.ground_direction;
    config.altitude_coefficient = param.environment_config.wind.altitude_coefficient;
    config.turbulence_intensity = param.environment_config.wind.turbulence_intensity;
    
    return config;
}

OutputConfig create_output_config(const Parameter& param) {
    OutputConfig config;
    config.format = param.output_config.format;
    config.directory = param.output_config.output_directory;
    config.filename_prefix = param.simulation.name;
    config.coordinate_system = param.output_config.coordinate_system;
    config.buffer_size = 1000;
    config.precision = 6;
    config.write_statistics = true;
    
    return config;
}

Physics::RigidBodyState create_initial_state(const Parameter& param) {
    Physics::RigidBodyState state;
    
    // Position (convert from lat/lon/alt to local coordinates for simple case)
    state.position = Physics::Vector3D(0.0, 0.0, param.launch.altitude);
    
    // Initial velocity (launch with specified elevation and azimuth)
    double elevation_rad = param.launch.elevation * M_PI / 180.0;
    double azimuth_rad = param.launch.azimuth * M_PI / 180.0;
    double initial_speed = 1.0; // Small initial velocity
    
    state.velocity = Physics::Vector3D(
        initial_speed * std::cos(elevation_rad) * std::sin(azimuth_rad),
        initial_speed * std::cos(elevation_rad) * std::cos(azimuth_rad),
        initial_speed * std::sin(elevation_rad)
    );
    
    // Orientation (initially aligned with velocity vector)
    state.orientation = Physics::Quaternion::identity();
    
    // No initial rotation
    state.angular_velocity = Physics::Vector3D(0.0, 0.0, 0.0);
    
    // Initialize mass properties
    state.mass = param.rocket_config.mass_total;
    
    // Inertia tensor (diagonal for cylinder)
    double radius = param.rocket_config.diameter / 2.0;
    double length = param.rocket_config.length;
    double ixx_iyy = state.mass * (3 * radius * radius + length * length) / 12.0;
    double izz = state.mass * radius * radius / 2.0;
    
    state.inertia_tensor = Physics::Matrix3x3::diagonal(ixx_iyy, ixx_iyy, izz);
    
    return state;
}

void print_statistics(const Simulation::Statistics& stats) {
    std::cout << "\n" << std::string(80, '=') << std::endl;
    std::cout << "📊 Simulation Statistics:" << std::endl;
    std::cout << std::string(80, '=') << std::endl;
    std::cout << "Flight Time: " << std::fixed << std::setprecision(2) << stats.flight_time << " s" << std::endl;
    std::cout << "Max Altitude: " << std::fixed << std::setprecision(1) << stats.max_altitude << " m" << std::endl;
    std::cout << "Max Velocity: " << std::fixed << std::setprecision(1) << stats.max_velocity << " m/s" << std::endl;
    std::cout << "Max Acceleration: " << std::fixed << std::setprecision(1) << stats.max_acceleration << " m/s²" << std::endl;
    std::cout << "Max Mach Number: " << std::fixed << std::setprecision(2) << stats.max_mach << std::endl;
    std::cout << "Burnout Time: " << std::fixed << std::setprecision(2) << stats.burnout_time << " s" << std::endl;
    std::cout << "Apogee Time: " << std::fixed << std::setprecision(2) << stats.apogee_time << " s" << std::endl;
    std::cout << "Integration Steps: " << stats.integration_steps << std::endl;
    std::cout << "Average Step Size: " << std::scientific << std::setprecision(3) << stats.average_step_size << " s" << std::endl;
    
    if (stats.recovery_deployed) {
        std::cout << "Recovery Deployment: " << std::fixed << std::setprecision(2) << stats.recovery_deployment_time << " s" << std::endl;
        std::cout << "Recovery Altitude: " << std::fixed << std::setprecision(1) << stats.recovery_deployment_altitude << " m" << std::endl;
    }
    
    std::cout << std::string(80, '=') << std::endl;
}

int main(int argc, char* argv[]) {
    print_header();

    // Parameter file handling
    std::string param_file = "parameter.toml";
    if (argc >= 2) {
        param_file = argv[1];
    }

    // Check if parameter file exists
    if (!std::filesystem::exists(param_file)) {
        std::cerr << "❌ Parameter file not found: " << param_file << std::endl;
        std::cerr << "Usage: " << argv[0] << " [parameter_file.toml]" << std::endl;
        return 1;
    }

    try {
        // Load parameters
        print_progress("Loading parameters", 0.1);
        Parameter param(param_file);
        if (!param.loadParameters()) {
            std::cerr << "\n❌ Error loading parameters from: " << param_file << std::endl;
            return 1;
        }
        print_progress("Loading parameters", 1.0);
        std::cout << std::endl;

        // Print configuration
        print_configuration(param);

        // Create output directory
        std::string output_dir = param.output_config.output_directory;
        if (!std::filesystem::exists(output_dir)) {
            std::filesystem::create_directories(output_dir);
            std::cout << "\n📁 Created output directory: " << output_dir << std::endl;
        }

        // Initialize components
        print_progress("Initializing components", 0.2);
        
        auto engine_config = create_engine_config(param);
        auto vehicle_config = create_vehicle_config(param);
        auto propulsion_config = create_propulsion_config(param);
        auto atmosphere_config = create_atmosphere_config(param);
        auto output_config = create_output_config(param);
        
        print_progress("Initializing components", 0.4);
        
        // Create vehicle with propulsion system
        auto vehicle = std::make_shared<Rocket::Vehicle>(vehicle_config);
        vehicle->get_propulsion_system().configure(propulsion_config);
        
        print_progress("Initializing components", 0.6);
        
        // Create environment
        auto atmosphere = std::make_shared<Environment::StandardAtmosphereModel>(atmosphere_config);
        
        print_progress("Initializing components", 0.8);
        
        // Create output manager
        auto output_manager = std::make_shared<Output::OutputManager>(output_config);
        
        print_progress("Initializing components", 1.0);
        std::cout << std::endl;

        // Create simulation engine
        std::cout << "\n🔧 Creating simulation engine..." << std::endl;
        Simulation::Engine engine(engine_config, vehicle, atmosphere, output_manager);

        // Set initial state
        auto initial_state = create_initial_state(param);
        engine.set_initial_state(initial_state);

        // Run simulation
        std::cout << "\n🚀 Starting simulation..." << std::endl;
        auto start_time = std::chrono::high_resolution_clock::now();
        
        bool success = engine.run();
        
        auto end_time = std::chrono::high_resolution_clock::now();
        auto duration = std::chrono::duration_cast<std::chrono::milliseconds>(end_time - start_time);

        if (success) {
            std::cout << "\n✅ Simulation completed successfully!" << std::endl;
            std::cout << "Execution Time: " << duration.count() << " ms" << std::endl;
            
            // Print statistics
            auto stats = engine.get_statistics();
            print_statistics(stats);
            
            std::cout << "\n💾 Output saved to: " << output_dir << std::endl;
        } else {
            std::cout << "\n❌ Simulation failed!" << std::endl;
            return 1;
        }

    } catch (const std::exception& e) {
        std::cerr << "\n❌ Error: " << e.what() << std::endl;
        return 1;
    } catch (...) {
        std::cerr << "\n❌ Unknown error occurred!" << std::endl;
        return 1;
    }

    return 0;
}
