#ifndef PARAMETER_H
#define PARAMETER_H

#include "rocket.hpp"
#include "environment.hpp"
#include "output.hpp"
#include <string>

class Parameter {
public:
    std::string param_file;
    Rocket rocket;
    Environment environment;
    OutputFormat output_format;

    Parameter(const std::string& filename);
    bool loadParameters();
};

#endif
