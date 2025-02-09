#ifndef PARAMETER_H
#define PARAMETER_H

#include "rocket.h"
#include "environment.h"
#include "output.h"  // OutputFormat の定義を使う

#define PARAM_FILE "parameter.txt"

int read_parameters(Rocket *r, Environment *env, OutputFormat *format);

#endif
