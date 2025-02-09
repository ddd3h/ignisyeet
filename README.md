# 🚀 IgnisYeet - 6-DOF Rocket Simulation with ECEF & Latitude-Longitude Output

## **Overview**
**IgnisYeet** is a **6-degree-of-freedom (6-DOF) rocket simulation** that models both translational and rotational motion. This simulation supports **ECEF (Earth-Centered, Earth-Fixed) coordinates** and **latitude-longitude-altitude representation**, configurable via a parameter file.

## **Features**
✅ **6-DOF Dynamics**: Simulates both position and orientation changes.  
✅ **Leapfrog Integration**: Efficient numerical integration for accurate motion prediction.  
✅ **ECEF & Latitude-Longitude Support**: Converts between Cartesian and geodetic coordinates.  
✅ **Configurable Parameters**: Modify initial conditions and simulation settings via `parameter.txt`.  
✅ **CSV Output**: Saves trajectory data for analysis.  
✅ **Progress Bar**: Displays real-time simulation progress in the terminal.  

---
## **Installation & Compilation**
1. Clone this repository:
   ```sh
   git clone https://github.com/ddd3h/ignisyeet.git
   cd ignisyeet
   ```

2. Compile the program using `make`:
   ```sh
   make
   ```

3. Run the simulation:
   ```sh
   ./ignisyeet
   ```

4. Clean up compiled files if necessary:
   ```sh
   make clean
   ```

---
## **File Structure**
```
./
│── src/
│   ├── main.c            # Main simulation loop
│   ├── rocket.c          # Rocket physics calculations
│   ├── parameter.c       # Reads input parameters
│   ├── output.c          # Handles CSV output
│
│── include/
│   ├── rocket.h          # Rocket definitions
│   ├── environment.h     # Environmental parameters
│   ├── parameter.h       # Input parameter definitions
│   ├── output.h          # Output handling functions
│
│── parameter.txt        # Configuration file
│── output.csv           # Simulation results
│── Makefile             # Compilation instructions
│── README.md            # This file
```

---
## **Configuration (`parameter.txt`)
Modify `parameter.txt` to adjust simulation settings.
Example:
```
# Initial Conditions
initial_x 0.0
initial_y 0.0
initial_z 1000.0   # Altitude above sea level (meters)
initial_vx 0.0
initial_vy 0.0
initial_vz 50.0
mass 100.0
thrust 500.0
thrust_duration 3.0

# Environmental Conditions
wind_x 0.0
wind_y 0.0
wind_z 0.0
air_density 1.23
gravity 9.81
drag_coef 0.5
area 0.1
time_step 0.01

# Output Format (ECEF or LATLON)
output_format LATLON
```

---
## **Output Format**
The results are saved in `output.csv` with the following format:

### **If `output_format ECEF` is selected:**
```
time,x,y,z,vz
0.00,1000000.00,2000000.00,3000000.00,50.00
...
```

### **If `output_format LATLON` is selected:**
```
time,latitude,longitude,altitude,vz
0.00,35.6895,139.6917,1000.00,50.00
...
```

---
## **Future Improvements**
🔹 Implement real-time visualization of rocket trajectory.  
🔹 Improve aerodynamics modeling for more accurate simulation.  
🔹 Add GUI support for interactive parameter adjustments.  

---
## **Contact**
For any issues or feature requests, please submit a GitHub issue or contact `daisukenishihama63@gmail.com`.

🚀 Welcome to **IgnisYeet** – where we launch fire and hope it lands safely! 🔥

