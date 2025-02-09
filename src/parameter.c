#include "parameter.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int read_parameters(Rocket *r, Environment *env, OutputFormat *format) {
    FILE *file = fopen(PARAM_FILE, "r");
    if (file == NULL) {
        printf("Error: Cannot open parameter file.\n");
        return 0;
    }

    char key[50];
    while (fscanf(file, "%s", key) != EOF) {
        if (strcmp(key, "initial_x") == 0) fscanf(file, "%lf", &r->Xe[0]);
        else if (strcmp(key, "initial_y") == 0) fscanf(file, "%lf", &r->Xe[1]);
        else if (strcmp(key, "initial_z") == 0) fscanf(file, "%lf", &r->Xe[2]);
        else if (strcmp(key, "initial_vx") == 0) fscanf(file, "%lf", &r->Ve[0]);
        else if (strcmp(key, "initial_vy") == 0) fscanf(file, "%lf", &r->Ve[1]);
        else if (strcmp(key, "initial_vz") == 0) fscanf(file, "%lf", &r->Ve[2]);
        else if (strcmp(key, "mass") == 0) fscanf(file, "%lf", &r->mass);
        else if (strcmp(key, "thrust") == 0) fscanf(file, "%lf", &r->thrust);
        else if (strcmp(key, "thrust_duration") == 0) fscanf(file, "%lf", &r->thrust_duration);
        else if (strcmp(key, "wind_x") == 0) fscanf(file, "%lf", &env->wind[0]);
        else if (strcmp(key, "wind_y") == 0) fscanf(file, "%lf", &env->wind[1]);
        else if (strcmp(key, "wind_z") == 0) fscanf(file, "%lf", &env->wind[2]);
        else if (strcmp(key, "air_density") == 0) fscanf(file, "%lf", &env->air_density);
        else if (strcmp(key, "gravity") == 0) fscanf(file, "%lf", &env->gravity);
        else if (strcmp(key, "drag_coef") == 0) fscanf(file, "%lf", &env->drag_coef);
        else if (strcmp(key, "area") == 0) fscanf(file, "%lf", &env->area);
        else if (strcmp(key, "time_step") == 0) fscanf(file, "%lf", &env->dt);
        else if (strcmp(key, "output_format") == 0) {
            char value[10];
            fscanf(file, "%s", value);
            if (strcmp(value, "ECEF") == 0) {
                *format = FORMAT_ECEF;
            } else if (strcmp(value, "LATLON") == 0) {
                *format = FORMAT_LATLON;
            } else {
                printf("Warning: Unknown output_format '%s', defaulting to ECEF.\n", value);
                *format = FORMAT_ECEF; // デフォルトは ECEF
            }
        }
    }

    fclose(file);
    return 1;
}
