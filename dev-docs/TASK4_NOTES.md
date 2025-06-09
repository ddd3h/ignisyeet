# Task 4: 重力モデル実装 メモ

開始日: 2025年6月9日

## 目標
IgnisYeet 6DOF シミュレーションシステムに段階的な重力モデルを実装し、精度レベルを選択可能にする。

## 実装計画

### 4.1 一様重力場 (Level 1) - 優先度: 高
**対象**: 基本的なロケット飛行シミュレーション
**概要**: 地表付近での一定重力加速度 (g = 9.81 m/s²) 
**実装**:
- `GravityModel` 基底クラス設計
- `UniformGravity` クラス実装
- パラメータファイル対応 (`gravity_level = 1`)

### 4.2 高度依存重力 (Level 2) - 優先度: 中
**対象**: 高高度ロケット (>10km)
**概要**: 地球中心からの距離に応じた重力変化
**式**: g(r) = g0 * (R_earth / r)²
**実装**:
- `AltitudeDependentGravity` クラス
- 地球半径パラメータ対応

### 4.3 楕円体地球+コリオリ力 (Level 3) - 優先度: 低
**対象**: 高精度軌道計算・衛星ミッション
**概要**: 地球の楕円体形状とコリオリ力を考慮
**実装**:
- `RotatingEarthGravity` クラス
- 地球楕円体パラメータ
- コリオリ力・遠心力計算

### 4.4 パラメータ切り替え機能
**実装**:
- `gravity_level` パラメータでモデル選択
- 実行時動的切り替え対応
- バリデーション・警告システム

## パラメータファイル拡張

```toml
[gravity]
level = 1  # 1: 一様, 2: 高度依存, 3: 楕円体+コリオリ
g0 = 9.81  # [m/s²] 地表重力加速度
earth_radius = 6371000.0  # [m] 地球半径
earth_rotation_rate = 7.2921159e-5  # [rad/s] 地球自転角速度
```

## クラス設計

```cpp
// 基底クラス
class GravityModel {
public:
    virtual Vector3D calculate_gravity(const Vector3D& position, 
                                     const Vector3D& velocity) const = 0;
    virtual int get_level() const = 0;
};

// Level 1: 一様重力
class UniformGravity : public GravityModel {
private:
    double g0_;
public:
    Vector3D calculate_gravity(const Vector3D& position, 
                             const Vector3D& velocity) const override;
    int get_level() const override { return 1; }
};

// Level 2: 高度依存
class AltitudeDependentGravity : public GravityModel {
private:
    double g0_, earth_radius_;
public:
    Vector3D calculate_gravity(const Vector3D& position, 
                             const Vector3D& velocity) const override;
    int get_level() const override { return 2; }
};

// Level 3: 楕円体+コリオリ
class RotatingEarthGravity : public GravityModel {
private:
    double g0_, earth_radius_, rotation_rate_;
    // 楕円体パラメータ等
public:
    Vector3D calculate_gravity(const Vector3D& position, 
                             const Vector3D& velocity) const override;
    int get_level() const override { return 3; }
};
```

## 統合ポイント

### Engine クラス統合
- `SimulationEngine` に `GravityModel` ポインタ追加
- `calculate_dynamics()` 内で重力計算呼び出し
- パラメータ読み込み時のモデル選択

### State クラス拡張
- 必要に応じて `RocketState` に重力関連情報追加
- コリオリ力のための座標系情報

## テスト戦略

### 4.1 単体テスト
- 各重力モデルの個別検証
- 既知の物理条件での結果確認

### 4.2 統合テスト  
- エンジンとの統合動作確認
- パラメータ切り替え動作検証

### 4.3 物理検証
- 自由落下時間の理論値比較
- 高度別重力変化の妥当性確認

## 実装順序

1. **Level 1 実装** (本日)
   - GravityModel 基底クラス
   - UniformGravity クラス
   - パラメータ統合
   - 基本テスト

2. **Level 2 実装** (明日)
   - AltitudeDependentGravity クラス
   - 高度計算ロジック
   - 統合テスト

3. **Level 3 実装** (明後日以降)
   - RotatingEarthGravity クラス
   - 座標変換ロジック
   - 高精度テスト

## 注意事項

- 座標系の一貫性維持（現在は地心固定座標系想定）
- 数値精度と計算コストのバランス
- 既存コードとの後方互換性
- エラーハンドリングの統一

## 参考資料

- 軌道力学の基礎理論
- 地球楕円体パラメータ (WGS84等)
- コリオリ力の数値計算手法

---

**現在のフォーカス**: Level 1 一様重力実装から開始
