使用例とサンプル
================

基本例
------

シンプルなロケット打ち上げ
~~~~~~~~~~~~~~~~~~~~~~~~

垂直打ち上げのシミュレーション例です。

.. code-block:: cpp

   #include "ignis/rocket.h"
   #include "ignis/physics/all.h"
   #include <iostream>
   #include <fstream>

   using namespace ignis;

   int main() {
       // 初期条件設定
       RocketState rocket;
       rocket.position = Vector3D(0, 0, 0);      // 発射台
       rocket.velocity = Vector3D(0, 0, 0);      // 静止状態
       rocket.mass = 50000.0;                    // 50トン
       
       // 物理モデル選択（Level 2）
       auto gravity = std::make_unique<AltitudeDependentGravity>();
       auto atmosphere = std::make_unique<ISAAtmosphere>();
       auto aerodynamics = std::make_unique<MachDependentModel>();
       
       // シミュレーションパラメータ
       double dt = 0.1;        // 100msステップ
       double thrust = 750000; // 750kN推力
       double burn_time = 120; // 2分間燃焼
       
       // 結果保存用ファイル
       std::ofstream output("trajectory.csv");
       output << "time,altitude,velocity,mach,acceleration\\n";
       
       // シミュレーションループ
       for (double t = 0; t < 300; t += dt) {
           Vector3D forces(0, 0, 0);
           
           // 推力（燃焼時間内のみ）
           if (t < burn_time) {
               forces.z += thrust;
               rocket.mass -= 300 * dt; // 燃料消費：300kg/s
           }
           
           // 重力
           forces += gravity->calculateForce(rocket);
           
           // 空気抵抗
           double air_density = atmosphere->getDensity(rocket.position.z);
           Vector3D drag = aerodynamics->calculateForce(rocket, air_density);
           forces += drag;
           
           // 状態更新
           Vector3D acceleration = forces / rocket.mass;
           rocket.velocity += acceleration * dt;
           rocket.position += rocket.velocity * dt;
           
           // 結果出力
           double mach = rocket.velocity.magnitude() / 
                        atmosphere->getSoundSpeed(rocket.position.z);
           
           output << t << "," << rocket.position.z << "," 
                  << rocket.velocity.magnitude() << "," << mach << ","
                  << acceleration.magnitude() << "\\n";
           
           // 地面に落下したら終了
           if (rocket.position.z < 0 && t > burn_time) break;
       }
       
       output.close();
       std::cout << "シミュレーション完了。結果をtrajectory.csvに保存しました。\\n";
       
       return 0;
   }

弾道軌道シミュレーション
~~~~~~~~~~~~~~~~~~~~~~

角度を持った打ち上げのシミュレーション例です。

.. code-block:: cpp

   #include "ignis/rocket.h"
   #include "ignis/physics/all.h"
   #include <cmath>

   using namespace ignis;

   void simulateBallisticTrajectory(double launch_angle_deg, 
                                   double initial_velocity) {
       RocketState rocket;
       
       // 初期速度の設定（角度を考慮）
       double angle_rad = launch_angle_deg * M_PI / 180.0;
       rocket.position = Vector3D(0, 0, 100);  // 100m高台から発射
       rocket.velocity = Vector3D(
           initial_velocity * cos(angle_rad),  // 水平成分
           0,                                  // 横風なし
           initial_velocity * sin(angle_rad)   // 垂直成分
       );
       rocket.mass = 1000.0;  // 1トン
       
       // 高精度物理モデル（Level 3）
       auto gravity = std::make_unique<RotatingEarthGravity>();
       auto atmosphere = std::make_unique<DynamicAtmosphere>();
       auto aerodynamics = std::make_unique<CompressibleFlowModel>();
       
       double dt = 0.01;
       std::cout << "発射角度: " << launch_angle_deg << "度\\n";
       std::cout << "初期速度: " << initial_velocity << " m/s\\n\\n";
       
       for (double t = 0; t < 100; t += dt) {
           Vector3D forces(0, 0, 0);
           
           // 重力（コリオリ力込み）
           forces += gravity->calculateForce(rocket, t);
           
           // 空気抵抗（風効果込み）
           forces += aerodynamics->calculateForce(rocket, *atmosphere, t);
           
           // 状態更新
           rocket.updateState(forces, dt);
           
           // 着地判定
           if (rocket.position.z <= 0) {
               std::cout << "着地時間: " << t << " 秒\\n";
               std::cout << "到達距離: " << rocket.position.x << " m\\n";
               std::cout << "最終速度: " << rocket.velocity.magnitude() 
                        << " m/s\\n\\n";
               break;
           }
       }
   }

   int main() {
       // 各種角度でのシミュレーション
       std::vector<double> angles = {15, 30, 45, 60, 75};
       double v0 = 200; // 200m/s初速
       
       for (double angle : angles) {
           simulateBallisticTrajectory(angle, v0);
       }
       
       return 0;
   }

高度例
------

多段ロケットシミュレーション
~~~~~~~~~~~~~~~~~~~~~~~~~~

実際の多段ロケットを模擬した例です。

.. code-block:: cpp

   #include "ignis/rocket.h"
   #include "ignis/physics/all.h"
   #include "ignis/staging.h"

   using namespace ignis;

   struct Stage {
       double dry_mass;      // 空虚質量
       double fuel_mass;     // 燃料質量
       double thrust;        // 推力
       double burn_time;     // 燃焼時間
       double specific_impulse; // 比推力
   };

   int main() {
       // 3段ロケット定義
       std::vector<Stage> stages = {
           {5000, 45000, 800000, 120, 280},  // 第1段
           {2000, 15000, 200000, 180, 320},  // 第2段
           {500,  3000,  50000,  300, 350}   // 第3段
       };
       
       RocketState rocket;
       rocket.position = Vector3D(0, 0, 0);
       rocket.velocity = Vector3D(0, 0, 0);
       
       // 全段の質量合計
       rocket.mass = 0;
       for (const auto& stage : stages) {
           rocket.mass += stage.dry_mass + stage.fuel_mass;
       }
       rocket.mass += 1000; // ペイロード質量
       
       // 高精度物理モデル
       auto gravity = std::make_unique<RotatingEarthGravity>();
       auto atmosphere = std::make_unique<ISAAtmosphere>();
       auto aerodynamics = std::make_unique<CompressibleFlowModel>();
       
       double dt = 0.1;
       int current_stage = 0;
       double stage_burn_time = 0;
       
       std::ofstream output("multistage_trajectory.csv");
       output << "time,stage,altitude,velocity,mass,acceleration\\n";
       
       for (double t = 0; t < 1800; t += dt) { // 30分間
           Vector3D forces(0, 0, 0);
           
           // 現在段の推力
           if (current_stage < stages.size()) {
               const Stage& stage = stages[current_stage];
               
               if (stage_burn_time < stage.burn_time) {
                   // 推力計算
                   forces.z += stage.thrust;
                   
                   // 燃料消費
                   double fuel_rate = stage.fuel_mass / stage.burn_time;
                   rocket.mass -= fuel_rate * dt;
                   
                   stage_burn_time += dt;
               } else {
                   // 段分離
                   rocket.mass -= stages[current_stage].dry_mass;
                   current_stage++;
                   stage_burn_time = 0;
                   
                   std::cout << "第" << current_stage << "段分離 at t=" 
                            << t << "s, alt=" << rocket.position.z 
                            << "m\\n";
               }
           }
           
           // 物理力計算
           forces += gravity->calculateForce(rocket, t);
           
           if (rocket.position.z < 100000) { // 100km以下で空気抵抗
               forces += aerodynamics->calculateForce(rocket, *atmosphere);
           }
           
           // 状態更新
           Vector3D accel = forces / rocket.mass;
           rocket.velocity += accel * dt;
           rocket.position += rocket.velocity * dt;
           
           // データ出力
           output << t << "," << current_stage << "," 
                  << rocket.position.z << "," 
                  << rocket.velocity.magnitude() << ","
                  << rocket.mass << "," << accel.magnitude() << "\\n";
       }
       
       output.close();
       
       std::cout << "最終高度: " << rocket.position.z / 1000 << " km\\n";
       std::cout << "最終速度: " << rocket.velocity.magnitude() << " m/s\\n";
       
       return 0;
   }

パフォーマンス最適化例
~~~~~~~~~~~~~~~~~~~~

大規模シミュレーション向けの最適化例です。

.. code-block:: cpp

   #include "ignis/rocket.h"
   #include "ignis/performance.h"
   #include <thread>
   #include <vector>

   using namespace ignis;

   // 並列シミュレーション例
   void parallelMonteCarloSimulation() {
       const int num_simulations = 10000;
       const int num_threads = std::thread::hardware_concurrency();
       
       std::vector<std::thread> threads;
       std::vector<std::vector<double>> results(num_threads);
       
       auto worker = [&](int thread_id, int start_sim, int end_sim) {
           for (int sim = start_sim; sim < end_sim; ++sim) {
               // ランダムパラメータ生成
               RocketState rocket = generateRandomRocket(sim);
               
               // 軽量物理モデル（Level 1）で高速計算
               UniformGravity gravity;
               ConstantDensityAtmosphere atmosphere;
               BasicDragModel aerodynamics;
               
               // シミュレーション実行
               double max_altitude = runSimulation(rocket, gravity, 
                                                 atmosphere, aerodynamics);
               
               results[thread_id].push_back(max_altitude);
           }
       };
       
       // スレッド起動
       int sims_per_thread = num_simulations / num_threads;
       for (int i = 0; i < num_threads; ++i) {
           int start = i * sims_per_thread;
           int end = (i == num_threads - 1) ? num_simulations : 
                     start + sims_per_thread;
           
           threads.emplace_back(worker, i, start, end);
       }
       
       // スレッド完了待機
       for (auto& thread : threads) {
           thread.join();
       }
       
       // 結果統計処理
       std::vector<double> all_results;
       for (const auto& thread_results : results) {
           all_results.insert(all_results.end(), 
                             thread_results.begin(), 
                             thread_results.end());
       }
       
       // 統計出力
       double mean = calculateMean(all_results);
       double std_dev = calculateStdDev(all_results);
       
       std::cout << "モンテカルロシミュレーション結果:\\n";
       std::cout << "試行回数: " << num_simulations << "\\n";
       std::cout << "平均高度: " << mean << " m\\n";
       std::cout << "標準偏差: " << std_dev << " m\\n";
   }

設定ファイル使用例
----------------

parameter.tomlの設定例
~~~~~~~~~~~~~~~~~~~~~

.. code-block:: toml

   # IgnisYeet設定ファイル
   
   [simulation]
   time_step = 0.01
   max_time = 600.0
   output_interval = 1.0
   
   [rocket]
   initial_mass = 25000.0
   dry_mass = 5000.0
   thrust = 400000.0
   burn_time = 180.0
   drag_coefficient = 0.4
   reference_area = 3.14
   
   [gravity]
   model_level = 2  # 高度依存重力
   surface_gravity = 9.81
   earth_radius = 6371000.0
   
   [atmosphere]
   model_level = 2  # ISA標準大気
   sea_level_density = 1.225
   sea_level_pressure = 101325.0
   sea_level_temperature = 288.15
   
   [aerodynamics]
   model_level = 2  # マッハ数依存
   enable_lift = true
   compressibility_correction = true

C++での設定使用例：

.. code-block:: cpp

   #include "ignis/configuration.h"
   
   int main() {
       try {
           Configuration config("parameter.toml");
           
           // 設定値取得
           double dt = config.getDouble("simulation.time_step");
           double max_time = config.getDouble("simulation.max_time");
           
           // ロケット設定
           double mass = config.getDouble("rocket.initial_mass");
           double thrust = config.getDouble("rocket.thrust");
           
           // 物理モデル自動生成
           auto models = config.createPhysicsModels();
           
           // シミュレーション実行
           runSimulation(config, models);
           
       } catch (const std::exception& e) {
           std::cerr << "設定エラー: " << e.what() << std::endl;
           return 1;
       }
       
       return 0;
   }
