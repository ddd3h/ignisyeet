#include "parameter.hpp"
#include "toml.hpp"
#include <iostream>
#include <fstream>
#include <stdexcept>

Parameter::Parameter(const std::string& filename) : param_file(filename) {
    // Initialize default values
    simulation = {};
    rocket_config = {};
    aerodynamics = {};
    propulsion = {};
    launch = {};
    environment_config = {};
    recovery = {};
    output_config = {};
    monte_carlo = {};
}

bool Parameter::loadParameters() {
    try {
        auto config = toml::parse_file(param_file);
        
        std::cout << "Loading parameters from: " << param_file << std::endl;
        
        // Load all configuration sections
        if (!loadSimulationConfig(config)) return false;
        if (!loadRocketConfig(config)) return false;
        if (!loadAerodynamicsConfig(config)) return false;
        if (!loadPropulsionConfig(config)) return false;
        if (!loadLaunchConfig(config)) return false;
        if (!loadEnvironmentConfig(config)) return false;
        if (!loadRecoveryConfig(config)) return false;
        if (!loadOutputConfig(config)) return false;
        if (!loadMonteCarloConfig(config)) return false;
        
        // Validate loaded parameters
        if (!validateParameters()) {
            std::cerr << "Parameter validation failed!" << std::endl;
            return false;
        }
        
        std::cout << "✅ Parameters loaded successfully!" << std::endl;
        return true;
        
    } catch (const toml::parse_error& err) {
        std::cerr << "❌ Error parsing TOML file: " << err << std::endl;
        return false;
    } catch (const std::exception& e) {
        std::cerr << "❌ Error loading parameters: " << e.what() << std::endl;
        return false;
    }
}

bool Parameter::loadSimulationConfig(const toml::table& config) {
    if (auto sim = config["simulation"].as_table()) {
        simulation.name = sim->at("name").value_or("default_simulation");
        simulation.description = sim->at("description").value_or("");
        simulation.max_time = sim->at("max_time").value_or(300.0);
        simulation.time_step = sim->at("time_step").value_or(0.01);
        simulation.adaptive_step = sim->at("adaptive_step").value_or(true);
        simulation.tolerance = sim->at("tolerance").value_or(1e-6);
        simulation.output_interval = sim->at("output_interval").value_or(0.1);
        
        simulation.terminate_on_ground = sim->at("terminate_on_ground").value_or(true);
        simulation.terminate_on_max_time = sim->at("terminate_on_max_time").value_or(true);
        simulation.terminate_on_apogee_descent = sim->at("terminate_on_apogee_descent").value_or(false);
        
        simulation.integration_method = sim->at("integration_method").value_or("rk4");
    }
    return true;
}

bool Parameter::loadRocketConfig(const toml::table& config) {
    if (auto rocket_tbl = config["rocket"].as_table()) {
        rocket_config.name = rocket_tbl->at("name").value_or("default_rocket");
        rocket_config.configuration_type = rocket_tbl->at("configuration_type").value_or("single_stage");
        
        // Geometry
        rocket_config.length = rocket_tbl->at("length").value_or(2.0);
        rocket_config.diameter = rocket_tbl->at("diameter").value_or(0.15);
        rocket_config.reference_area = rocket_tbl->at("reference_area").value_or(0.0177);
        
        // Mass properties
        rocket_config.mass_dry = rocket_tbl->at("mass_dry").value_or(7.0);
        rocket_config.mass_propellant = rocket_tbl->at("mass_propellant").value_or(3.0);
        rocket_config.mass_total = rocket_tbl->at("mass_total").value_or(10.0);
        
        // Center of gravity
        rocket_config.cg_initial = rocket_tbl->at("cg_initial").value_or(1.0);
        rocket_config.cg_final = rocket_tbl->at("cg_final").value_or(0.8);
        rocket_config.cg_propellant = rocket_tbl->at("cg_propellant").value_or(0.5);
        
        // Load inertia arrays
        if (auto inertia_init = rocket_tbl->at("inertia_initial").as_array()) {
            rocket_config.inertia_initial.clear();
            for (auto&& elem : *inertia_init) {
                if (auto val = elem.as_floating_point()) {
                    rocket_config.inertia_initial.push_back(val->get());
                }
            }
        }
        
        if (auto inertia_final = rocket_tbl->at("inertia_final").as_array()) {
            rocket_config.inertia_final.clear();
            for (auto&& elem : *inertia_final) {
                if (auto val = elem.as_floating_point()) {
                    rocket_config.inertia_final.push_back(val->get());
                }
            }
        }
        
        // CAD configuration
        if (auto cad = rocket_tbl->at("cad").as_table()) {
            rocket_config.cad.enable = cad->at("enable").value_or(false);
            rocket_config.cad.stl_file = cad->at("stl_file").value_or("");
            rocket_config.cad.auto_calculate_inertia = cad->at("auto_calculate_inertia").value_or(false);
            rocket_config.cad.auto_calculate_cg = cad->at("auto_calculate_cg").value_or(false);
        }
    }
    return true;
}

bool Parameter::loadAerodynamicsConfig(const toml::table& config) {
    if (auto aero = config["aerodynamics"].as_table()) {
        aerodynamics.model_level = aero->at("model_level").value_or(2);
        
        // Reference geometry
        aerodynamics.reference_area = aero->at("reference_area").value_or(0.0177);
        aerodynamics.reference_length = aero->at("reference_length").value_or(1.0);
        aerodynamics.center_of_pressure = aero->at("center_of_pressure").value_or(0.6);
        aerodynamics.nose_cone_angle = aero->at("nose_cone_angle").value_or(0.2);
        aerodynamics.fin_area = aero->at("fin_area").value_or(0.05);
        
        // Basic drag
        aerodynamics.cd_constant = aero->at("cd_constant").value_or(0.5);
        aerodynamics.cd_mach_dependent = aero->at("cd_mach_dependent").value_or(true);
        aerodynamics.cd_table_file = aero->at("cd_table_file").value_or("");
        
        // Lift and lateral forces
        aerodynamics.enable_lift = aero->at("enable_lift").value_or(true);
        aerodynamics.enable_lateral = aero->at("enable_lateral").value_or(true);
        aerodynamics.cl_alpha = aero->at("cl_alpha").value_or(3.5);
        aerodynamics.cy_beta = aero->at("cy_beta").value_or(-2.0);
        
        // Moment coefficients
        aerodynamics.cm_alpha = aero->at("cm_alpha").value_or(-0.02);
        aerodynamics.cn_beta = aero->at("cn_beta").value_or(0.15);
        aerodynamics.cl_p = aero->at("cl_p").value_or(-0.5);
        
        // Pressure center
        aerodynamics.cp_position = aero->at("cp_position").value_or(1.1);
        
        // Compressibility effects
        aerodynamics.enable_compressibility = aero->at("enable_compressibility").value_or(true);
        aerodynamics.mach_critical = aero->at("mach_critical").value_or(0.8);
        aerodynamics.mach_drag_rise = aero->at("mach_drag_rise").value_or(1.2);
    }
    return true;
}

bool Parameter::loadPropulsionConfig(const toml::table& config) {
    if (auto prop = config["propulsion"].as_table()) {
        propulsion.engine_type = prop->at("engine_type").value_or("solid");
        
        // Thrust characteristics
        propulsion.thrust_profile_type = prop->at("thrust_profile_type").value_or("file");
        propulsion.thrust_profile_file = prop->at("thrust_profile_file").value_or("");
        propulsion.thrust_constant = prop->at("thrust_constant").value_or(500.0);
        propulsion.burn_time = prop->at("burn_time").value_or(3.0);
        
        // Performance parameters
        propulsion.isp_vacuum = prop->at("isp_vacuum").value_or(250.0);
        propulsion.isp_sea_level = prop->at("isp_sea_level").value_or(220.0);
        propulsion.throat_area = prop->at("throat_area").value_or(0.0007);
        propulsion.exit_area = prop->at("exit_area").value_or(0.0042);
        propulsion.expansion_ratio = prop->at("expansion_ratio").value_or(6.0);
        
        // Chamber conditions
        propulsion.chamber_pressure = prop->at("chamber_pressure").value_or(2.0e6);
        propulsion.exit_pressure = prop->at("exit_pressure").value_or(1.0e5);
    }
    return true;
}

bool Parameter::loadLaunchConfig(const toml::table& config) {
    if (auto launch_tbl = config["launch"].as_table()) {
        // Launch site
        launch.latitude = launch_tbl->at("latitude").value_or(35.6895);
        launch.longitude = launch_tbl->at("longitude").value_or(139.6917);
        launch.altitude = launch_tbl->at("altitude").value_or(100.0);
        
        // Launch conditions
        launch.azimuth = launch_tbl->at("azimuth").value_or(90.0);
        launch.elevation = launch_tbl->at("elevation").value_or(85.0);
        launch.rail_length = launch_tbl->at("rail_length").value_or(3.0);
        launch.rail_exit_velocity = launch_tbl->at("rail_exit_velocity").value_or(10.0);
        
        // Load initial velocity array
        if (auto init_vel = launch_tbl->at("initial_velocity").as_array()) {
            launch.initial_velocity.clear();
            for (auto&& elem : *init_vel) {
                if (auto val = elem.as_floating_point()) {
                    launch.initial_velocity.push_back(val->get());
                }
            }
        }
        
        // Load initial angular velocity array
        if (auto init_omega = launch_tbl->at("initial_angular_velocity").as_array()) {
            launch.initial_angular_velocity.clear();
            for (auto&& elem : *init_omega) {
                if (auto val = elem.as_floating_point()) {
                    launch.initial_angular_velocity.push_back(val->get());
                }
            }
        }
    }
    return true;
}

bool Parameter::loadEnvironmentConfig(const toml::table& config) {
    if (auto env = config["environment"].as_table()) {
        // Gravity model
        if (auto gravity = env->at("gravity").as_table()) {
            environment_config.gravity.level = gravity->at("level").value_or(1);
            environment_config.gravity.g0 = gravity->at("g0").value_or(9.80665);
            environment_config.gravity.earth_radius = gravity->at("earth_radius").value_or(6371000.0);
            environment_config.gravity.earth_rotation_rate = gravity->at("earth_rotation_rate").value_or(7.2921159e-5);
            environment_config.gravity.enable_coriolis = gravity->at("enable_coriolis").value_or(false);
            environment_config.gravity.enable_centrifugal = gravity->at("enable_centrifugal").value_or(false);
        }
        
        // Atmosphere model
        if (auto atm = env->at("atmosphere").as_table()) {
            environment_config.atmosphere.level = atm->at("level").value_or(2);
            
            // Level 1: Simple atmosphere
            if (auto simple = atm->at("simple").as_table()) {
                environment_config.atmosphere.simple.density = simple->at("density").value_or(1.225);
                environment_config.atmosphere.simple.temperature = simple->at("temperature").value_or(288.15);
                environment_config.atmosphere.simple.pressure = simple->at("pressure").value_or(101325.0);
            }
            
            // Level 2+: Wind settings
            if (auto wind = atm->at("wind").as_table()) {
                if (auto vel = wind->at("velocity").as_array()) {
                    environment_config.atmosphere.wind.velocity.clear();
                    for (auto&& elem : *vel) {
                        if (auto val = elem.as_floating_point()) {
                            environment_config.atmosphere.wind.velocity.push_back(val->get());
                        }
                    }
                }
            }
            
            // Level 3: Dynamic atmosphere
            if (auto gust = atm->at("gust").as_table()) {
                environment_config.atmosphere.gust.intensity = gust->at("intensity").value_or(5.0);
                environment_config.atmosphere.gust.frequency = gust->at("frequency").value_or(0.1);
                
                if (auto scale = gust->at("turbulence_scale").as_array()) {
                    environment_config.atmosphere.gust.turbulence_scale.clear();
                    for (auto&& elem : *scale) {
                        if (auto val = elem.as_floating_point()) {
                            environment_config.atmosphere.gust.turbulence_scale.push_back(val->get());
                        }
                    }
                }
            }
        }
    }
    return true;
}

bool Parameter::loadRecoveryConfig(const toml::table& config) {
    if (auto recovery_tbl = config["recovery"].as_table()) {
        recovery.enable_parachute = recovery_tbl->at("enable_parachute").value_or(true);
        recovery.deployment_altitude = recovery_tbl->at("deployment_altitude").value_or(500.0);
        recovery.deployment_time = recovery_tbl->at("deployment_time").value_or(-1.0);
        recovery.drag_coefficient_chute = recovery_tbl->at("drag_coefficient_chute").value_or(1.3);
        recovery.chute_area = recovery_tbl->at("chute_area").value_or(2.0);
        recovery.descent_rate_target = recovery_tbl->at("descent_rate_target").value_or(5.0);
    }
    return true;
}

bool Parameter::loadOutputConfig(const toml::table& config) {
    if (auto out = config["output"].as_table()) {
        output_config.primary_format = out->at("primary_format").value_or("csv");
        output_config.output_directory = out->at("output_directory").value_or("./results");
        output_config.filename_prefix = out->at("filename_prefix").value_or("simulation");
        output_config.include_timestamp = out->at("include_timestamp").value_or(true);
        output_config.include_config_hash = out->at("include_config_hash").value_or(false);
        output_config.coordinate_system = out->at("coordinate_system").value_or("latlon");
        
        // Load secondary formats array
        if (auto sec_formats = out->at("secondary_formats").as_array()) {
            output_config.secondary_formats.clear();
            for (auto&& elem : *sec_formats) {
                if (auto val = elem.as_string()) {
                    output_config.secondary_formats.push_back(val->get());
                }
            }
        }
        
        // Load output fields
        if (auto fields = out->at("fields").as_table()) {
            output_config.fields.time = fields->at("time").value_or(true);
            output_config.fields.position = fields->at("position").value_or(true);
            output_config.fields.velocity = fields->at("velocity").value_or(true);
            output_config.fields.acceleration = fields->at("acceleration").value_or(true);
            output_config.fields.attitude = fields->at("attitude").value_or(true);
            output_config.fields.angular_velocity = fields->at("angular_velocity").value_or(true);
            output_config.fields.forces = fields->at("forces").value_or(true);
            output_config.fields.moments = fields->at("moments").value_or(true);
            output_config.fields.environment = fields->at("environment").value_or(true);
            output_config.fields.propulsion = fields->at("propulsion").value_or(true);
        }
    }
    return true;
}

bool Parameter::loadMonteCarloConfig(const toml::table& config) {
    if (auto mc = config["monte_carlo"].as_table()) {
        monte_carlo.enable = mc->at("enable").value_or(false);
        monte_carlo.num_runs = mc->at("num_runs").value_or(1000);
        monte_carlo.random_seed = mc->at("random_seed").value_or(42);
        
        // Parameter variations will be implemented later
    }
    return true;
}

bool Parameter::validateParameters() {
    // Basic validation checks
    if (simulation.time_step <= 0) {
        std::cerr << "❌ Invalid time step: " << simulation.time_step << std::endl;
        return false;
    }
    
    if (rocket_config.mass_total <= 0) {
        std::cerr << "❌ Invalid rocket mass: " << rocket_config.mass_total << std::endl;
        return false;
    }
    
    if (rocket_config.length <= 0 || rocket_config.diameter <= 0) {
        std::cerr << "❌ Invalid rocket dimensions" << std::endl;
        return false;
    }
    
    // Validate integration method
    const std::vector<std::string> valid_methods = {"euler", "leapfrog", "rk4", "rk45"};
    if (std::find(valid_methods.begin(), valid_methods.end(), simulation.integration_method) == valid_methods.end()) {
        std::cerr << "❌ Invalid integration method: " << simulation.integration_method << std::endl;
        return false;
    }
    
    std::cout << "✅ Parameter validation passed" << std::endl;
    return true;
}

void Parameter::printConfiguration() {
    std::cout << "\n" << std::string(80, '=') << std::endl;
    std::cout << "🚀 IgnisYeet Simulation Configuration" << std::endl;
    std::cout << std::string(80, '=') << std::endl;
    
    std::cout << "Simulation: " << simulation.name << std::endl;
    std::cout << "Description: " << simulation.description << std::endl;
    std::cout << "Integration Method: " << simulation.integration_method << std::endl;
    std::cout << "Time Step: " << simulation.time_step << " s" << std::endl;
    std::cout << "Max Time: " << simulation.max_time << " s" << std::endl;
    
    std::cout << "\nRocket Configuration:" << std::endl;
    std::cout << "  Name: " << rocket_config.name << std::endl;
    std::cout << "  Mass: " << rocket_config.mass_total << " kg" << std::endl;
    std::cout << "  Length: " << rocket_config.length << " m" << std::endl;
    std::cout << "  Diameter: " << rocket_config.diameter << " m" << std::endl;
    
    std::cout << "\nLaunch Site:" << std::endl;
    std::cout << "  Latitude: " << launch.latitude << "°" << std::endl;
    std::cout << "  Longitude: " << launch.longitude << "°" << std::endl;
    std::cout << "  Altitude: " << launch.altitude << " m" << std::endl;
    
    std::cout << std::string(80, '=') << std::endl;
}
