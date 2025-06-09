開発者向けガイド
================

このセクションでは、IgnisYeetの開発に参加する際のガイドラインと、
新しい物理モデルの追加方法について説明します。

開発環境のセットアップ
--------------------

必要なツール
~~~~~~~~~~~~

* C++17対応コンパイラ
* CMakeまたはMake
* Git
* Doxygen（ドキュメント生成用）
* Catch2（テスト用）
* Sphinx + Breathe（ドキュメント生成用）

開発フロー
~~~~~~~~~~

1. リポジトリをフォーク
2. feature/your-feature-name ブランチを作成
3. 変更を実装
4. テストを追加・実行
5. ドキュメントを更新
6. プルリクエストを作成

コーディング規約
----------------

命名規則
~~~~~~~~

.. code-block:: cpp

   // クラス名：PascalCase
   class RocketState;
   class AerodynamicModel;
   
   // 関数名・変数名：snake_case
   double calculateDrag();
   Vector3D rocket_position;
   
   // 定数：UPPER_SNAKE_CASE
   const double EARTH_RADIUS = 6371000.0;
   
   // 名前空間：小文字
   namespace ignis {
       // ...
   }

ファイル構成
~~~~~~~~~~~~

.. code-block:: text

   src/
   ├── ignis/
   │   ├── core/          # コアクラス
   │   ├── physics/       # 物理モデル
   │   ├── utils/         # ユーティリティ
   │   └── configuration/ # 設定システム
   tests/
   ├── unit/              # 単体テスト
   ├── integration/       # 統合テスト
   └── performance/       # 性能テスト

新しい物理モデルの追加
--------------------

基本手順
~~~~~~~~

1. 基底クラスを継承したクラスを作成
2. 必要なメソッドをオーバーライド
3. 対応するテストを作成
4. ドキュメントを追加
5. 設定システムに統合

例: 新しい重力モデルの追加
~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: cpp

   // src/ignis/physics/gravity/custom_gravity.h
   #pragma once
   
   #include "ignis/physics/gravity_model.h"
   
   namespace ignis {
   
   /**
    * @brief カスタム重力モデル
    * 
    * 独自の重力計算を実装したモデル。
    * 特殊な天体や実験的な重力理論に対応。
    */
   class CustomGravityModel : public GravityModel {
   public:
       /**
        * @brief コンストラクタ
        * @param custom_param カスタムパラメータ
        */
       explicit CustomGravityModel(double custom_param);
       
       /**
        * @brief 重力ベクトル計算
        * @param rocket_state ロケット状態
        * @param time 時刻
        * @return 重力ベクトル [N]
        */
       Vector3D calculateForce(const RocketState& rocket_state, 
                              double time = 0.0) const override;
       
       /**
        * @brief モデル名取得
        * @return モデル名
        */
       std::string getModelName() const override {
           return "CustomGravity";
       }
       
       /**
        * @brief モデルレベル取得
        * @return レベル（4以上の拡張レベル）
        */
       int getModelLevel() const override {
           return 4;
       }
   
   private:
       double custom_param_;  ///< カスタムパラメータ
       
       /**
        * @brief 内部計算ヘルパー関数
        */
       double calculateCustomEffect(const Vector3D& position) const;
   };
   
   } // namespace ignis

対応するテストの作成：

.. code-block:: cpp

   // tests/unit/test_custom_gravity.cpp
   #include <catch2/catch_test_macros.hpp>
   #include "ignis/physics/gravity/custom_gravity.h"
   #include "ignis/core/rocket_state.h"
   
   using namespace ignis;
   
   TEST_CASE("CustomGravityModel基本テスト", "[gravity][custom]") {
       CustomGravityModel gravity(1.0);
       
       SECTION("基本的な重力計算") {
           RocketState rocket;
           rocket.position = Vector3D(0, 0, 1000);
           rocket.mass = 1000.0;
           
           Vector3D force = gravity.calculateForce(rocket);
           
           REQUIRE(force.z < 0);  // 下向きの力
           REQUIRE(force.magnitude() > 0);
       }
       
       SECTION("高度依存性テスト") {
           RocketState rocket1, rocket2;
           rocket1.position = Vector3D(0, 0, 1000);
           rocket2.position = Vector3D(0, 0, 10000);
           rocket1.mass = rocket2.mass = 1000.0;
           
           Vector3D force1 = gravity.calculateForce(rocket1);
           Vector3D force2 = gravity.calculateForce(rocket2);
           
           // 高高度では重力が小さくなることを確認
           REQUIRE(force2.magnitude() < force1.magnitude());
       }
   }

設定システムへの統合
~~~~~~~~~~~~~~~~~~

.. code-block:: cpp

   // src/ignis/configuration/physics_factory.cpp
   
   std::unique_ptr<GravityModel> 
   PhysicsFactory::createGravityModel(const Configuration& config) {
       int level = config.getInt("gravity.model_level");
       
       switch (level) {
           case 1:
               return std::make_unique<UniformGravity>();
           case 2:
               return std::make_unique<AltitudeDependentGravity>();
           case 3:
               return std::make_unique<RotatingEarthGravity>();
           case 4:  // 新しいモデルを追加
               double param = config.getDouble("gravity.custom_param");
               return std::make_unique<CustomGravityModel>(param);
           default:
               throw std::invalid_argument("不正な重力モデルレベル");
       }
   }

テスト戦略
----------

単体テスト
~~~~~~~~~~

各物理モデルは以下の観点でテストを行います：

* **物理的妥当性**: 物理法則に従った結果か
* **境界条件**: 極限値での動作
* **数値安定性**: 数値誤差の蓄積
* **パフォーマンス**: 計算速度

.. code-block:: cpp

   TEST_CASE("物理的妥当性テスト", "[physics][validation]") {
       // エネルギー保存則チェック
       // 運動量保存則チェック
       // 次元解析チェック
   }

統合テスト
~~~~~~~~~~

複数の物理モデルを組み合わせた統合テストを実施します：

.. code-block:: cpp

   TEST_CASE("物理モデル統合テスト", "[integration]") {
       // 重力 + 大気 + 空力の組み合わせテスト
       // 既知の解析解との比較
       // 実験データとの照合
   }

パフォーマンステスト
~~~~~~~~~~~~~~~~~~

計算性能の監視とベンチマークを行います：

.. code-block:: cpp

   TEST_CASE("パフォーマンステスト", "[performance]") {
       auto start = std::chrono::high_resolution_clock::now();
       
       // 大量計算実行
       for (int i = 0; i < 1000000; ++i) {
           model.calculateForce(rocket);
       }
       
       auto end = std::chrono::high_resolution_clock::now();
       auto duration = std::chrono::duration_cast<std::chrono::microseconds>
                      (end - start);
       
       // 性能要件チェック
       REQUIRE(duration.count() < 1000000); // 1秒以内
   }

ドキュメント作成
----------------

コードドキュメント
~~~~~~~~~~~~~~~~

Doxygenスタイルでコメントを記述します：

.. code-block:: cpp

   /**
    * @brief 関数の簡潔な説明
    * 
    * より詳しい説明。使用例や注意点も記載。
    * 
    * @param input_param 入力パラメータの説明 [単位]
    * @param optional_param オプションパラメータ（デフォルト値あり）
    * @return 戻り値の説明 [単位]
    * 
    * @throws std::invalid_argument 不正な引数の場合
    * @throws std::runtime_error 計算エラーの場合
    * 
    * @see 関連する関数やクラス
    * @note 特記事項
    * @warning 警告事項
    * 
    * @example
    * @code
    * Model model(1.0);
    * Vector3D result = model.calculate(input);
    * @endcode
    */

Sphinxドキュメント
~~~~~~~~~~~~~~~~~

RSTファイルでユーザーガイドを作成します：

.. code-block:: rst

   新機能の使用方法
   ================
   
   概要
   ----
   
   新機能の概要説明。
   
   使用例
   ------
   
   .. code-block:: cpp
   
      // C++コード例
      NewModel model;
      auto result = model.process();

継続的インテグレーション
----------------------

GitHub Actions設定例：

.. code-block:: yaml

   # .github/workflows/ci.yml
   name: CI
   
   on: [push, pull_request]
   
   jobs:
     test:
       runs-on: ubuntu-latest
       
       steps:
       - uses: actions/checkout@v3
       
       - name: Setup dependencies
         run: |
           sudo apt-get update
           sudo apt-get install -y build-essential cmake
       
       - name: Build
         run: make all
       
       - name: Run tests
         run: make test
       
       - name: Generate documentation
         run: make doc

リリース プロセス
-----------------

1. **バージョン番号更新**
2. **変更ログの作成**
3. **包括的テスト実行**
4. **ドキュメント更新**
5. **タグ作成・リリース**

貢献ガイドライン
----------------

プルリクエストを提出する前に：

* [ ] 全テストが通過することを確認
* [ ] 新機能にはテストを追加
* [ ] ドキュメントを更新
* [ ] コーディング規約に準拠
* [ ] コミットメッセージは明確で簡潔に
