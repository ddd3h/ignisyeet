.. _page-theory-overview:

モデルの全体像
================

ロケットの飛翔シミュレーションは、飛翔体の運動を予測する計算である。
主な用途は次の三つである。

* 飛翔経路（到達高度、最大速度、最大動圧など）の予測
* 落下位置の推定と、その風による広がり（落下分散）の評価
* 保安域の設定など、安全対策の基礎資料の作成（手順は :ref:`sec-dispersion-safety`）

この章では、IgnisYeet のモデルが何で構成され、設定ファイルのどの項目がどのモデルを選ぶのかを説明する。
各モデルの選び方は、:ref:`page-theory` の各章にある。

この章で決めること
--------------------------

全体として決めるのは、何を計算したいか（目的）と、その目的に必要な精度と計算時間である。
目的ごとのコマンドと、選択を左右する主な設定を :numref:`tbl-choose-purpose` に示す。

選び方
------

.. _tbl-choose-purpose:

.. list-table:: 目的別のコマンドと主な設定
   :header-rows: 1
   :widths: 20 14 36 30

   * - 目的
     - コマンド
     - 選択を左右する主な設定
     - 判断の章
   * - 飛翔経路（頂点、最大速度、着地点）を 1 本求める
     - ``sim``
     - ``aero.method``、``earth.model``、``wind.model``、``sim.integrator``
     - :ref:`page-theory-aero-choose`、:ref:`page-theory-wind`、:ref:`page-theory-integrators`
   * - 空力係数の表だけを求める、外部の係数表と比べる
     - ``aero``
     - ``aero.method``、``aero.table_kind``、``aero.cfd.*``
     - :ref:`page-theory-aero-choose`、:ref:`page-theory-aero-tables`
   * - 風速と風向を変えたときの着地点の広がりを見通す
     - ``dispersion``\ （``wind_grid``）
     - ``dispersion.wind_speeds``、``dispersion.directions``、``wind.model``
     - :ref:`page-theory-dispersion`
   * - 機体と環境の不確かさを含めた着地点の分布を求める
     - ``dispersion``\ （``monte_carlo``）
     - ``dispersion.monte_carlo.*``\ （サンプル数、標準偏差）
     - :ref:`page-theory-dispersion`
   * - 保安距離を決める
     - ``dispersion`` の両方
     - ``aero.method``\ （手法間の差）、サンプル数、``truncate_sigma``
     - :ref:`page-howto-keepout-zone`、:ref:`page-theory-dispersion`

設定ブロックごとの選択肢と、それを説明する章の対応は :numref:`fig-opt-map` にある。

計算時間の目安を :numref:`tbl-overview-time` に示す。計算時間は、空力係数の手法（特に CFD）で桁が変わる。
手法の選択は、精度の要求とこの計算時間の釣り合いで決める。

.. _tbl-overview-time:

.. list-table:: サンプル機体での計算時間（20 スレッドの CPU）
   :header-rows: 1
   :widths: 50 50

   * - 処理
     - 時間
   * - 形状抽出（約 1.4 万三角形、600 断面）＋係数表（151 Mach × 91 迎角）
     - 0.1 s 未満（係数表だけなら約 20 ms）
   * - 係数表（パネル法、``aero.method = "panel"``、既定のメッシュ約 5900 枚）
     - 約 9 s（亜音速 6 点の連立方程式が約 5 s）。CPU が他の処理と競合すると長くなる（:ref:`sec-vaero-timing`）
   * - 係数表（CFD、``aero.method = "cfd"``、Euler、約 142 万セル（フィンの面 1 mm）、:math:`M=0.3`〜2 の 4 点 × 迎角 4 点の 16 ケース）
     - 行列全体（16 ケース）で約 2.5 時間（4 ランクで 1 ケースずつ、別の計算と同時に実行した測定。1 ケースは約 6〜15 分。:ref:`sec-vaero-timing`）
   * - 1 回の飛翔（パラシュート降下を含め約 690 s 分）、``rk4``\ （:math:`\Delta t=2` ms）
     - 約 0.2 s
   * - 同、``rk45``\ （既定の許容誤差）
     - 約 0.03 s
   * - 落下分散 ``wind_grid``\ （7 風速 × 8 方位 × 2 降下モード = 112 ケース、``rk4``）
     - 約 0.7 s
   * - 落下分散 ``monte_carlo``\ （1000 サンプル × 2 降下モード、``rk45``）
     - 約 0.8 s

測定の条件と内訳は :ref:`sec-vaero-timing` にまとめてある。

モデル
------

.. _fig-pipeline:

.. figure:: /_generated/tikz/pipeline.*
   :width: 100%

   IgnisYeet のモデルの構成。形状から係数表（空力モデル）を作り、環境と推力を与えた飛翔計算がそれを引き、落下分散は飛翔計算を多数のケースで繰り返す。青が Rust、橙が Python で実装した部分である。

IgnisYeet は次のモデルで構成される。入力は機体の STL、設定ファイル、推力曲線の三つである。

**形状モデル**\ （:ref:`page-theory-geometry`）
   STL を機軸に垂直な多数の断面で切り、胴体の半径分布とフィンの寸法を求める。胴体は軸対称、フィンは台形と仮定する。

**空力モデル**\ （:ref:`page-theory-aero-choose`）
   求めた形状から、Mach 数 :math:`M` と迎角 :math:`\alpha` の格子上で空力係数を計算し、表として保存する。
   手法は設定で選べ、既定は部品積み上げ法（Barrowman 法、:ref:`page-theory-barrowman`）、``aero.method = "panel"`` で表面パネルを解くパネル法（:ref:`page-theory-panel`）、
   ``aero.method = "cfd"`` で SU2 による CFD（:ref:`page-theory-cfd-choose`）、``aero.method = "table"`` で外部の係数表になる。

**環境モデルと推進モデル**\ （:ref:`page-theory-coordinates`、:ref:`page-theory-atmosphere-gravity`、:ref:`page-theory-wind`、:ref:`page-theory-propulsion`）
   座標系（平面地球と自転する ECEF）、大気（米国標準大気 :cite:`us76`\ 、温度差、一定大気）、重力（一定・逆 2 乗則・:math:`J_2`）、風（一定・べき法則・対数則・高度別の表）、
   推力曲線と質量特性を与える。

**飛翔モデル**\ （:ref:`page-theory-dynamics`、:ref:`page-theory-descent`、:ref:`page-theory-integrators`）
   6 自由度の運動方程式を、空力係数を各時刻の :math:`(M,\alpha)` で表から内挿して積分する。
   積分法は固定刻みの 4 次 Runge–Kutta 法と、刻み幅を自動で調整する Dormand–Prince 法 :cite:`dormand` から選ぶ :cite:`hairer`。

**落下分散モデル**\ （:ref:`page-theory-dispersion`）
   風速と風向の組合せごとに（``wind_grid``）、または推力・質量・空力係数・射角・風などをランダムに変えたサンプルごとに（``monte_carlo``。モンテカルロ法 :cite:`metropolis`）、
   飛翔を独立に計算し、着地点を集める。

作図（Python）は、出力 CSV から図を作る。

空力係数を先に表にする理由
~~~~~~~~~~~~~~~~~~~~~~~~~~

飛翔計算は、各時刻で空力係数を何度も評価する。数百秒の飛翔や、100 ケースを超える落下分散では、その回数が非常に多くなる。
空力係数そのものは形状と :math:`(M,\alpha)` だけで決まり、飛翔の途中で形状は変わらない。
そこで、係数を格子上で一度だけ計算して表にしておき、飛翔計算では表の内挿だけを行う。
内挿は四つの格子点の重み付き和なので非常に速く、空力モデルの計算コストと飛翔計算のコストを切り離せる。
より重い空力解析に置き換える場合も、同じ表形式で出力すれば飛翔計算側は変更せずに使える。
パネル法と CFD は同じ表を作り、外部の表も ``aero.method = "table"`` で読み込める。

主な力
~~~~~~

ロケットに働く主な力は **推力・重力・空気力** である。

推力
   最も大きな力で、軌道を大きく左右する。燃焼試験などで得た推力履歴（推力曲線）をそのまま用いる。推力の一般論は教科書 :cite:`sutton` に詳しい。

重力
   地上付近ではほぼ一定とみなせるが、高度が上がると小さくなる。既定では地球中心からの距離の 2 乗に反比例するモデルを用いる（:ref:`sec-gravity`）。

空気力
   機体が空気中を進むことで生じる力で、機体軸方向の **軸力** と、それに垂直な **法線力** に分けて扱う。
   大きさは機体形状・Mach 数・迎角に依存し、さらに風によって対気速度が変わるため、軌道への影響が大きい。
   空力係数の推算は、細長いロケット向けの部品積み上げ法 :cite:`barrowman` を基礎にし、パネル法と CFD も選べる。

モデルロケット向けのオープンソースの飛翔シミュレータには OpenRocket :cite:`niskanen,niskanentech` があり、IgnisYeet の空力係数と抗力の推算法はこれに倣っている。

適用範囲と限界
--------------

モデルの適用範囲は、各章の「適用範囲と限界」と :ref:`page-theory-limitations` にある。
結果の妥当性は、仮定が対象の機体と飛翔条件で成り立つかどうかに依存する。結果を使う前に :ref:`page-theory-limitations` を確かめる。

精度の証拠
----------

各モデルの検証・相互比較・妥当性確認は :ref:`page-vv` にまとめてある。

* 環境、座標変換、運動方程式、積分法、落下分散は、解析解や定義との比較で検証している（:ref:`page-vv-verification`）。
* 空力モデル（部品積み上げ法、パネル法、CFD）は、手法どうしの相互比較とメッシュの収束で検証している（:ref:`page-vv-aero-verification`、:ref:`page-vv-cross-comparison`）。
* 実測との比較（妥当性確認）は、Basic Finner の超音速（Mach 1.08–4.30）の射場試験に限られる（:ref:`page-vv-validation-basic-finner`）。

関連する設定キー
----------------

.. list-table::
   :header-rows: 1
   :widths: 36 24 40

   * - キー
     - 既定値
     - 選ぶ章
   * - :ref:`cfg-aero-method`
     - ``barrowman``
     - :ref:`page-theory-aero-choose`
   * - :ref:`cfg-earth-model`
     - ``flat``
     - :ref:`page-theory-coordinates`
   * - :ref:`cfg-atmosphere-model`
     - ``us1976``
     - :ref:`page-theory-atmosphere-gravity`
   * - :ref:`cfg-wind-model`
     - ``power``
     - :ref:`page-theory-wind`
   * - :ref:`cfg-sim-integrator`
     - ``rk4``
     - :ref:`page-theory-integrators`
   * - :ref:`cfg-dispersion-mode`
     - ``wind_grid``\ （サンプル）
     - :ref:`page-theory-dispersion`

設定キーの全体は :ref:`page-reference-config` にある。
