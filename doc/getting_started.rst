スタートガイド
==============

インストール
------------

前提条件
~~~~~~~~

* C++17対応コンパイラ（GCC 8+, Clang 7+, MSVC 2019+）
* CMakeまたはMake
* Catch2テストフレームワーク（オプション）

ビルド手順
~~~~~~~~~~

.. code-block:: bash

   # リポジトリクローン
   git clone https://github.com/username/ignisyeet.git
   cd ignisyeet

   # ビルド
   make all

   # テスト実行
   make test

基本的な使用方法
----------------

シンプルなロケットシミュレーション
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: cpp

   #include "ignis/rocket.h"
   #include "ignis/physics/gravity.h"
   #include "ignis/physics/atmosphere.h"
   #include "ignis/physics/aerodynamics.h"

   using namespace ignis;

   int main() {
       // ロケット状態の初期化
       RocketState rocket;
       rocket.position = Vector3D(0, 0, 0);      // 地表
       rocket.velocity = Vector3D(0, 0, 100);    // 上向き100m/s
       rocket.mass = 1000.0;                     // 1000kg

       // 物理モデルの設定
       auto gravity = std::make_unique<AltitudeDependentGravity>();
       auto atmosphere = std::make_unique<ISAAtmosphere>();
       auto aerodynamics = std::make_unique<MachDependentModel>();

       // シミュレーション実行
       double dt = 0.01;  // 10msステップ
       for (int i = 0; i < 10000; ++i) {
           Vector3D forces = Vector3D(0, 0, 0);
           
           // 重力計算
           forces += gravity->calculateForce(rocket);
           
           // 空気抵抗計算
           forces += aerodynamics->calculateForce(rocket, *atmosphere);
           
           // 状態更新
           rocket.updateState(forces, dt);
           
           // 結果出力
           if (i % 100 == 0) {
               std::cout << "時刻: " << i * dt << "s, "
                        << "高度: " << rocket.position.z << "m" << std::endl;
           }
       }
       
       return 0;
   }

設定ファイルの使用
~~~~~~~~~~~~~~~~

`parameter.toml`ファイルでシミュレーションパラメータを設定：

.. code-block:: toml

   [gravity]
   model_level = 2  # 高度依存重力モデル
   surface_gravity = 9.81

   [atmosphere]
   model_level = 2  # ISA標準大気モデル
   
   [aerodynamics]
   model_level = 2  # マッハ数依存モデル
   drag_coefficient = 0.5
   reference_area = 1.0

.. code-block:: cpp

   #include "ignis/configuration.h"
   
   // 設定読み込み
   Configuration config("parameter.toml");
   
   // 物理モデル自動選択
   auto physics = config.createPhysicsModels();

次のステップ
------------

* :doc:`physics_models` - 各物理モデルの詳細
* :doc:`api_reference` - APIリファレンス
* :doc:`examples` - より詳細な使用例
