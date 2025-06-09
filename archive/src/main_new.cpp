#include "rocket.hpp"
#include "parameter.hpp"
#include "output.hpp"
#include <iostream>
#include <iomanip>
#include <thread>
#include <chrono>
#include <filesystem>

void print_progress(double progress) {
    int bar_width = 50;
    int pos = static_cast<int>(progress * bar_width);

    std::cout << "\r[";
    for (int i = 0; i < bar_width; i++) {
        if (i < pos) std::cout << "=";
        else if (i == pos) std::cout << ">";
        else std::cout << " ";
    }
    std::cout << "] " << std::fixed << std::setprecision(1) << (progress * 100.0) << "%" << std::flush;
    std::this_thread::sleep_for(std::chrono::milliseconds(50));
}

int main(int argc, char* argv[]) {
    std::cout << "\n" << std::string(80, '=') << std::endl;
    std::cout << "🚀 IgnisYeet - 6DOF Rocket Simulation v2.0" << std::endl;
    std::cout << std::string(80, '=') << std::endl;

    // Parameter file handling
    std::string param_file = "parameter.toml";  // Default parameter file
    if (argc >= 2) {
        param_file = argv[1];
    }

    // Check if parameter file exists
    if (!std::filesystem::exists(param_file)) {
        std::cerr << "❌ Parameter file not found: " << param_file << std::endl;
        std::cerr << "Usage: " << argv[0] << " [parameter_file.toml]" << std::endl;
        return 1;
    }

    // Load parameters
    Parameter param(param_file);
    if (!param.loadParameters()) {
        std::cerr << "❌ Error loading parameters from: " << param_file << std::endl;
        return 1;
    }

    // Print configuration
    param.printConfiguration();

    // Create output directory if it doesn't exist
    std::string output_dir = param.output_config.output_directory;
    if (!std::filesystem::exists(output_dir)) {
        std::filesystem::create_directories(output_dir);
        std::cout << "📁 Created output directory: " << output_dir << std::endl;
    }

    // Initialize simulation based on new parameter structure
    std::cout << "\n🚀 Starting " << param.simulation.name << " simulation..." << std::endl;
    std::cout << "Integration Method: " << param.simulation.integration_method << std::endl;
    std::cout << "Time Step: " << param.simulation.time_step << " s" << std::endl;
    std::cout << "Max Time: " << param.simulation.max_time << " s" << std::endl;

    // Legacy compatibility - map new parameters to old structures
    // This will be removed once the rocket and environment classes are updated
    param.rocket.Xe[0] = param.launch.latitude;
    param.rocket.Xe[1] = param.launch.longitude;
    param.rocket.Xe[2] = param.launch.altitude;
    param.rocket.mass = param.rocket_config.mass_total;
    param.rocket.length = param.rocket_config.length;
    param.rocket.diameter = param.rocket_config.diameter;
    param.rocket.drag_coefficient = param.aerodynamics.cd_constant;
    
    param.environment.dt = param.simulation.time_step;
    param.environment.wind[0] = param.environment_config.wind.ground_speed;
    param.environment.wind[1] = param.environment_config.wind.ground_direction;

    // Set output format based on coordinate system
    if (param.output_config.coordinate_system == "ecef") {
        param.output_format = FORMAT_ECEF;
    } else {
        param.output_format = FORMAT_LATLON;
    }

    // Initialize output system
    initialize_csv(param.output_format);

    // Simulation main loop
    double time = 0.0;
    double max_time = param.simulation.max_time;
    double last_progress_update = 0.0;
    int step_count = 0;

    std::cout << "\n🏃 Running simulation..." << std::endl;
    auto start_time = std::chrono::high_resolution_clock::now();

    while (time < max_time) {
        // Check termination conditions
        bool should_terminate = false;
        
        if (param.simulation.terminate_on_ground && param.rocket.Xe[2] <= 0) {
            std::cout << "\n🛬 Rocket has landed at time: " << time << " s" << std::endl;
            should_terminate = true;
        }
        
        if (param.simulation.terminate_on_max_time && time >= max_time) {
            std::cout << "\n⏰ Maximum simulation time reached: " << time << " s" << std::endl;
            should_terminate = true;
        }
        
        if (should_terminate) break;

        // Compute rocket dynamics
        param.rocket.thrust = param.rocket.compute_thrust(time);
        param.rocket.update(param.environment, param.environment.dt);
        
        // Save data at specified intervals
        if (time - static_cast<int>(time / param.simulation.output_interval) * param.simulation.output_interval < param.environment.dt) {
            save_to_csv(param.output_format, time, 
                       param.rocket.Xe[0], param.rocket.Xe[1], param.rocket.Xe[2], 
                       param.rocket.Ve[2]);
        }

        // Update progress bar every second
        if (time - last_progress_update >= 1.0) {
            print_progress(time / max_time);
            last_progress_update = time;
        }

        time += param.environment.dt;
        step_count++;
    }

    // Final progress bar update
    print_progress(1.0);
    
    auto end_time = std::chrono::high_resolution_clock::now();
    auto duration = std::chrono::duration_cast<std::chrono::milliseconds>(end_time - start_time);

    std::cout << "\n\n" << std::string(80, '=') << std::endl;
    std::cout << "🏁 Simulation Complete!" << std::endl;
    std::cout << std::string(80, '=') << std::endl;
    std::cout << "Total Steps: " << step_count << std::endl;
    std::cout << "Final Time: " << time << " s" << std::endl;
    std::cout << "Execution Time: " << duration.count() << " ms" << std::endl;
    std::cout << "Final Altitude: " << param.rocket.Xe[2] << " m" << std::endl;
    std::cout << "Output saved to: " << output_dir << std::endl;
    std::cout << std::string(80, '=') << std::endl;

    return 0;
}
