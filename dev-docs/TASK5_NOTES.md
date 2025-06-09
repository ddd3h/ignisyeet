# Task 5: 大気モデル実装 - 設計ノート

作成日: 2025年6月9日

## 概要

IgnisYeetロケットシミュレーションシステムのTask 5として、包括的な大気モデルシステムを実装する。
3段階のレベル分け（Level 1-3）により、用途に応じた計算精度と複雑さを選択可能にする。

## 実装要件

### 5.1 大気モデルレベル設計

#### Level 1: 定密度大気 (Simple Atmosphere)
- **目的**: 基本的なシミュレーション、概算計算
- **特徴**: 
  - 一定密度 (ρ = 1.225 kg/m³, 海面高度標準)
  - 一定温度 (T = 288.15 K, 15°C)
  - 一定圧力 (P = 101325 Pa)
  - 風なし
- **計算負荷**: 最小

#### Level 2: ISA標準大気 (Standard Atmosphere)
- **目的**: 実用的なシミュレーション、工学計算
- **特徴**:
  - International Standard Atmosphere (ISA) 準拠
  - 高度依存密度・温度・圧力
  - 対流圏 (0-11km): 線形温度減少
  - 成層圏下部 (11-20km): 一定温度
  - 定常風対応（高度依存可能）
- **計算負荷**: 中程度

#### Level 3: 動的大気 (Dynamic Atmosphere)
- **目的**: 高精度シミュレーション、研究用途
- **特徴**:
  - ISA標準大気ベース
  - 風モデル統合（定常風 + 突風）
  - 乱流効果
  - 温度変動効果
  - 季節・地域依存パラメータ
- **計算負荷**: 最大

### 5.2 クラス設計

```cpp
namespace ignis {

// 抽象基底クラス
class AtmosphereModel {
public:
    struct AtmosphereState {
        double density;      // [kg/m³]
        double temperature;  // [K]
        double pressure;     // [Pa]
        Vector3D wind_velocity; // [m/s]
    };
    
    virtual ~AtmosphereModel() = default;
    virtual AtmosphereState calculate_atmosphere(
        const Vector3D& position,
        double time = 0.0
    ) const = 0;
    virtual int get_level() const = 0;
    virtual std::string get_name() const = 0;
};

// Level 1: 定密度大気
class SimpleAtmosphere : public AtmosphereModel {
private:
    double density_;
    double temperature_;
    double pressure_;
    
public:
    SimpleAtmosphere(double density = 1.225, 
                     double temperature = 288.15, 
                     double pressure = 101325.0);
    
    AtmosphereState calculate_atmosphere(
        const Vector3D& position, double time = 0.0
    ) const override;
    
    int get_level() const override { return 1; }
    std::string get_name() const override { return "Simple"; }
};

// Level 2: ISA標準大気
class StandardAtmosphere : public AtmosphereModel {
private:
    struct ISALayer {
        double altitude_base;    // [m]
        double temperature_base; // [K]
        double pressure_base;    // [Pa]
        double lapse_rate;       // [K/m]
    };
    
    std::vector<ISALayer> layers_;
    Vector3D constant_wind_;
    
    void initialize_isa_layers();
    
public:
    StandardAtmosphere(const Vector3D& wind = Vector3D(0, 0, 0));
    
    AtmosphereState calculate_atmosphere(
        const Vector3D& position, double time = 0.0
    ) const override;
    
    int get_level() const override { return 2; }
    std::string get_name() const override { return "Standard"; }
    
    void set_constant_wind(const Vector3D& wind) { constant_wind_ = wind; }
};

// Level 3: 動的大気
class DynamicAtmosphere : public StandardAtmosphere {
private:
    // 突風モデルパラメータ
    double gust_intensity_;     // 突風強度 [m/s]
    double gust_frequency_;     // 突風周波数 [Hz]
    Vector3D turbulence_scale_; // 乱流スケール [m]
    
    // 風モデル計算
    Vector3D calculate_wind(const Vector3D& position, double time) const;
    Vector3D calculate_gust(const Vector3D& position, double time) const;
    
public:
    DynamicAtmosphere(const Vector3D& wind = Vector3D(0, 0, 0),
                      double gust_intensity = 5.0,
                      double gust_frequency = 0.1);
    
    AtmosphereState calculate_atmosphere(
        const Vector3D& position, double time = 0.0
    ) const override;
    
    int get_level() const override { return 3; }
    std::string get_name() const override { return "Dynamic"; }
    
    void set_gust_parameters(double intensity, double frequency);
    void set_turbulence_scale(const Vector3D& scale);
};

} // namespace ignis
```

### 5.3 ISA標準大気実装詳細

#### 対流圏 (Troposphere): 0 - 11,000m
- 温度: T(h) = T₀ - L × h
- T₀ = 288.15 K (海面温度)
- L = 0.0065 K/m (温度逓減率)

#### 成層圏下部 (Lower Stratosphere): 11,000 - 20,000m  
- 温度: T = 216.65 K (一定)

#### 圧力計算 (静水圧平衡)
- 対流圏: P(h) = P₀ × (T(h)/T₀)^(g×M/(R×L))
- 成層圏: P(h) = P₁ × exp(-g×M×(h-h₁)/(R×T₁))

#### 密度計算 (理想気体法則)
- ρ(h) = P(h) × M / (R × T(h))

定数:
- g = 9.80665 m/s² (標準重力加速度)
- M = 0.0289644 kg/mol (空気のモル質量)
- R = 8.31447 J/(mol·K) (気体定数)

### 5.4 パラメータ設定

```toml
[environment.atmosphere]
level = 2                    # 大気モデルレベル (1-3)

# Level 1 設定
[environment.atmosphere.simple]
density = 1.225              # 密度 [kg/m³]
temperature = 288.15         # 温度 [K]
pressure = 101325.0          # 圧力 [Pa]

# Level 2+ 設定
[environment.atmosphere.wind]
velocity = [5.0, 0.0, 0.0]   # 定常風速度 [m/s] (East, North, Up)

# Level 3 設定
[environment.atmosphere.gust]
intensity = 5.0              # 突風強度 [m/s]
frequency = 0.1              # 突風周波数 [Hz]
turbulence_scale = [100.0, 100.0, 50.0] # 乱流スケール [m]
```

### 5.5 シミュレーションエンジン統合

```cpp
// Engine クラスへの統合
class Engine {
private:
    std::unique_ptr<ignis::AtmosphereModel> atmosphere_model_;
    
public:
    void create_atmosphere_model(const Parameters& params);
    
    void compute_forces_moments(State& state, double time) {
        // 大気状態計算
        auto atm_state = atmosphere_model_->calculate_atmosphere(
            state.position, time
        );
        
        // 空力計算に使用
        compute_aerodynamic_forces(state, atm_state);
    }
};
```

### 5.6 テスト計画

#### 単体テスト
1. **Level 1 テスト**: 定数値検証
2. **Level 2 テスト**: ISA標準値との比較
3. **Level 3 テスト**: 風・突風効果の検証

#### 統合テスト
1. **高度プロファイル**: 0-20km大気特性変化
2. **風影響**: 飛行軌道への風の影響評価
3. **性能比較**: レベル間の計算精度・速度比較

#### 物理検証
1. **密度変化**: 高度11kmで約1/4に減少
2. **温度変化**: 対流圏で約65K低下
3. **風効果**: 横風による軌道偏向

### 5.7 実装順序

1. **Phase 1**: 基本クラス構造とLevel 1実装
2. **Phase 2**: Level 2 ISA標準大気実装
3. **Phase 3**: Level 3 動的大気・風モデル実装
4. **Phase 4**: パラメータシステム統合
5. **Phase 5**: シミュレーションエンジン統合
6. **Phase 6**: 包括的テストシステム構築

## 実装メモ

### ヘッダーファイル構成
- `include/physics/atmosphere.hpp`: メインヘッダー（header-only実装）
- Eigenライブラリ依存（Vector3D使用）
- 数学定数・物理定数定義

### 依存関係
- `include/physics/vector3d.hpp`: Vector3D型
- `include/parameter.hpp`: パラメータ構造体
- `<cmath>`, `<vector>`: 標準ライブラリ

### パフォーマンス考慮
- Level 1: O(1) - 即座に計算
- Level 2: O(1) - 高度に基づく単純計算
- Level 3: O(1) - 三角関数計算含む

完了予定: 2-3日
