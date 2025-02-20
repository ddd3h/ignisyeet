#include "parameter.hpp"
#include "toml.hpp"
#include <iostream>
#include <fstream>

Parameter::Parameter(const std::string& filename) : param_file(filename) {}

bool Parameter::loadParameters() {
    try {
        auto config = toml::parse_file(param_file);

        // Launch parameters
        if (auto launch = config["launch"].as_table()) {
            rocket.Xe[0] = launch->at("latitude").value_or(0.0);
            rocket.Xe[1] = launch->at("longitude").value_or(0.0);
            rocket.Xe[2] = launch->at("altitude").value_or(0.0);
            rocket.launcher_azimuth = launch->at("launcher_azimuth").value_or(0.0);
            rocket.launcher_elevation = launch->at("launcher_elevation").value_or(0.0);
        }

        // Rocket parameters
        if (auto rocket_tbl = config["rocket"].as_table()) {
            rocket.length = rocket_tbl->at("length").value_or(0.0);
            rocket.diameter = rocket_tbl->at("diameter").value_or(0.0);
            rocket.mass = rocket_tbl->at("mass_initial").value_or(0.0);
            rocket.drag_coefficient = rocket_tbl->at("drag_coefficient").value_or(0.0);
        }

        // Wind & Environment
        if (auto env_tbl = config["wind"].as_table()) {
            environment.wind[0] = env_tbl->at("ground_speed").value_or(0.0);
            environment.wind[1] = env_tbl->at("ground_direction").value_or(0.0);
        }

        // Output Format
        std::string format = config["output"]["file_name"].value_or("simulation_output.csv");
        if (format == "ECEF")
            output_format = FORMAT_ECEF;
        else
            output_format = FORMAT_LATLON;

        return true;
    } catch (const toml::parse_error& err) {
        std::cerr << "Error parsing TOML: " << err << std::endl;
        return false;
    }
}
