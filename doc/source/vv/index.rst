.. _page-vv:

検証と妥当性確認
==================

この章は、IgnisYeet の各モデルについて、どの根拠で何を確かめたか、どこまで確かめていないかをまとめる。
係数表や飛翔の結果をどこまで信頼してよいかを判断するときに使う。

用語
------

この章では、比較の相手によって三つの語を区別する。

.. list-table:: 検証、相互比較、妥当性確認
   :header-rows: 1
   :widths: 18 36 46

   * - 語
     - 比べる相手
     - 分かること
   * - 検証
     - 解析解、手計算、文献の関係式、収束の次数
     - 実装が式どおりに動くか。モデルの仮定が実機に合うかは分からない
   * - 相互比較
     - 別の手法の計算結果（部品積み上げ法、パネル法、CFD）
     - 手法どうしの差の大きさ。どちらが実機に近いかは決まらない
   * - 妥当性確認
     - 実測（風洞、弾道射場）
     - モデルが実機の値をどれだけ再現するか。実測と比べた量、Mach 数、形状の範囲に限られる

「CFD で確認済み」のように、比べる相手を言わない書き方はしない。CFD や SU2 の結果は実測ではないので、妥当性確認の根拠にならない。

証拠の一覧
------------

:numref:`fig-vv-map` に、モデルごとの三つの比較の有無を示す。実測との比較（妥当性確認）がある量は、Basic Finner の超音速の範囲だけである。

.. _fig-vv-map:

.. figure:: /_generated/tikz/vv_map.*
   :width: 90%

   モデルと比較の種類の対応。青は「ある」、橙は「部分的」、灰は「なし」。右の列が、実測との比較がどのモデルにあるかを表す。「部分的」の範囲は下の表に示す。

.. list-table:: 証拠の一覧
   :header-rows: 1
   :widths: 18 14 40 28

   * - モデル
     - 種類
     - 比べた相手と結果の要約
     - ページ
   * - 形状抽出、標準大気、重力、風、測地座標
     - 検証
     - 設計寸法（フィン寸法の誤差 1 mm 以内）、文献値、閉じた式（往復誤差 :math:`10^{-4}` m 未満）と一致
     - :ref:`page-vv-verification`
   * - 運動方程式、積分法、事象、二段開傘
     - 検証
     - 真空中の解析解（頂点の差 0.002 %）、終端速度（5 % 以内）、収束次数（``dop853`` は 8）
     - :ref:`page-vv-verification`
   * - 非定常な空力（減衰、Magnus、誘導ロール）
     - 検証
     - 線形理論（固有振動数 0.001 %）、Murphy の理論、細長い物体の理論。実測のピッチ減衰との比較はない
     - :ref:`sec-verify-unsteady`
   * - 部品積み上げ法
     - 検証
     - 手計算（:math:`C_{N\alpha}` を 4 桁で再現）、ロール微係数の数値積分
     - :ref:`sec-vaero-barrowman`
   * - 係数表
     - 検証
     - 展開と保存の往復（:math:`10^{-8}`）
     - :ref:`sec-vaero-table`
   * - パネル法
     - 検証
     - 球、長球、Helmbold、Taylor–Maccoll（表面 :math:`C_p` の誤差 0.012 以下）
     - :ref:`sec-vaero-panel`
   * - CFD（Euler）
     - 検証
     - Taylor–Maccoll（メッシュ 2.5 mm で誤差 1.8 %）、メッシュ収束（既定と細密の差 0.14 %）
     - :ref:`page-vv-cfd-verification`
   * - CFD（RANS）
     - 検証
     - 平板の基準解と相関式（1.5 % 以内）、円すいの圧力、摩擦の見積もり、メッシュ収束（GCI 2 % 以内）
     - :ref:`page-vv-rans-verification`
   * - 三手法、パネル法対部品積み上げ法、STL 対再構成
     - 相互比較
     - 法線力傾斜の手法差（パネル法対部品積み上げ法で 10〜57 %）、STL と再構成の CFD の差（1〜5 %）
     - :ref:`page-vv-cross-comparison`
   * - CFD（RANS）対 Euler
     - 相互比較
     - :math:`C_N` が 3〜6 % 大きい
     - :ref:`sec-cfd-rans-compare`
   * - 部品積み上げ法、パネル法、Euler、RANS
     - 妥当性確認（部分的）
     - Basic Finner の Mach 1.08–4.30 の射場と風洞の値。Euler の :math:`C_{N\alpha}` は 3〜11 % 小さい。RANS は Mach 1.832 の 1 点だけ
     - :ref:`page-vv-validation-basic-finner`

計算時間とメモリの測定は :ref:`page-vv-performance` にある。

**妥当性確認の範囲。** 実測と比べたのは Basic Finner の Mach 1.08–4.30（弾道射場）とロール減衰（風洞）だけである。亜音速、遷音速の山、Basic Finner 以外の形状、実機の飛翔の係数、ピッチ減衰は、実測との比較を行っていない。
このため、どの手法の係数も実機の係数の保証ではない（:ref:`sec-lim-aero`）。

再現方法
----------

検証の結果は、次の方法で再現できる。

* 式と数値解法の検証（:ref:`page-vv-verification`、:ref:`page-vv-aero-verification`）：リポジトリの直下で ``cargo test --release`` を実行する。
* CFD と RANS の計算結果、Basic Finner の実測値と比較の表：保存した結果が ``doc/data/cfd/``、``doc/data/validation/basic_finner/``、``doc/data/perf/`` にある。表と図はこれらの値から作る。
* サンプル機体の係数表と飛翔の結果：``examples/`` の設定ファイルを :ref:`page-tutorial` の手順で実行する。
* 図とデータを作るスクリプトの一覧は開発者向けの章（:ref:`page-dev`）にある。

.. toctree::
   :maxdepth: 1

   verification
   aero_verification
   cfd_verification
   rans_verification
   cross_comparison
   performance
   validation_basic_finner
