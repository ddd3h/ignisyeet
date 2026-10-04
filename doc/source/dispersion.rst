落下分散
========

落下分散とは、風の条件によって着地点がどれだけ広がるかを示すものである。
保安域の設定など、打上げの安全を判断するための基本的な資料になる。

落下分散には、風だけを変える **風の格子**\ （``mode = "wind_grid"``、既定）と、
推力・質量・空力係数・射角・風などのばらつきを同時に与える **モンテカルロ** 解析（``mode = "monte_carlo"``）がある。
どちらも ``ignisyeet dispersion`` で計算する。

風の格子（wind_grid）
---------------------

計算方法
~~~~~~~~

基準風速の一覧 :math:`\{w_1,\dots,w_K\}`\ （既定 1〜7 m/s）と、北から等間隔の :math:`J` 方位（既定 8 方位）

.. math::

   \psi_{w,j} = \frac{360^\circ\,j}{J},\qquad j=0,1,\dots,J-1

のすべての組合せについて、飛翔計算を行い着地点を求める。
パラシュートが有効な場合は、正常に開傘した場合（パラシュート降下）と、開傘しなかった場合（弾道落下）の両方を計算する。
したがってケース数は :math:`K\times J\times2` である。

各ケースは互いに独立なので、Rust の ``rayon`` で全ケースを並列に計算する。
空力係数表は全ケースで共有し、読み取りだけを行うので、並列化による競合は起きない。
ケースごとの風は、``[wind]`` で選んだ風モデルの基準風速と基準風向だけを差し替えて作る。
高度別の表（``profile``）では、表の形を保ったまま拡大・回転する（:ref:`sec-wind-scaling`）。

結果の見方
~~~~~~~~~~

:numref:`fig-dispersion` は、同じ風速で風向だけを変えた着地点を線で結んだものである。
内側から外側へ、風速が大きくなる。

.. _fig-dispersion:

.. figure:: _generated/plots/dispersion.*
   :width: 100%

   サンプル機体の落下分散（射角 85°、射方位 270°、7 風速 × 8 方位）。左が弾道落下、右がパラシュート降下。

弾道落下（左）
   機体は風見効果で風上を向いて飛ぶので、着地点は **風上側** にずれる。
   多角形の中心が射点の西（射方位）にずれているのは、射角 85° で西へ打ち出しているためである。

パラシュート降下（右）
   機体は風に流されるので、着地点は **風下側** にずれる。降下時間が長い（頂点 4.4 km から約 10 分）ので、ずれは弾道落下より 1 桁大きい。
   サンプルでは 7 m/s で 14 km を超えるが、これはべき法則（:math:`n=6`）が上空の風速を大きく見積もるためでもある（:ref:`sec-wind`）。

頂点付近で機体がほぼ真上を向いている場合、弾道落下では頂点で機体が失速して回転し、落ちていく向きがわずかな条件の差で変わる。
このため、弾道落下の多角形には一部いびつな点が現れることがある。これは計算の誤りではなく、実際の物理的な挙動を反映したものである。
安全評価では、このような点を含めた最も外側の範囲で考えるべきである。

.. _sec-mc:

モンテカルロ解析（monte_carlo）
-------------------------------

実際の落下点は、風だけでなく推力のばらつき、機体の質量、空力係数の推算誤差、ランチャの設置誤差など多くの要因でばらつく。
モンテカルロ解析は、これらを確率変数として多数回の飛翔計算を行い、着地点の分布を統計量で表す :cite:`metropolis`。
サンプル数を :math:`N`\ （``samples``）として、サンプル :math:`i=0,\dots,N-1` ごとに次の手順を行う。

1. 11 個のばらつきを乱数で引く。
2. ばらつきを機体・推力・風・回収系に反映した飛翔計算を、弾道落下と（パラシュートが有効なら）パラシュート降下の両方について、**同じ乱数で** 行う。
3. 着地点などの結果を記録する。

サンプルは互いに独立なので、並列に計算する。結果は計算の順序によらず、出力はサンプル番号順（同じ番号では弾道落下が先）に並ぶ。

乱数
~~~~

サンプル :math:`i` の乱数生成器は、ChaCha8 :cite:`bernstein` を ``seed + i``\ （64 ビット整数の足し算で、あふれたら 0 に戻る）で初期化したものである。
したがって、サンプル :math:`i` の値はほかのサンプルの数や実行する順序、スレッド数に依存せず、同じ設定なら **毎回同じ結果** になる。
種は ``seed + i`` だけで決まるので、``seed`` が :math:`s` のサンプル :math:`i` と、``seed`` が :math:`s-1` のサンプル :math:`i+1` は同じ値を引く。

引く値は、次の 11 個の標準正規乱数 :math:`z_1,\dots,z_{11}`\ （:math:`z_k\sim N(0,1)`）に標準偏差 :math:`\sigma_k` を掛けたもの
:math:`\epsilon_k=\sigma_kz_k` である。標準正規乱数は Ziggurat 法 :cite:`marsaglia` で生成する。引く順序は :numref:`tbl-mc-perturb` の上から下の順で、
**標準偏差が 0 のものも含めて常に 11 個すべてを引く**。そのため、ある項目の標準偏差を 0 にしても（あるいは 0 でなくしても）、
ほかの項目の値は変わらない。1 項目だけの影響を調べるときに、サンプルどうしを比較できる。

ばらつきの与え方
~~~~~~~~~~~~~~~~

相対のばらつき :math:`\epsilon` は、公称値に次の係数を掛けて反映する。

.. math::
   :label: eq-mc-factor

   f(\epsilon)=\max\,(1+\epsilon,\ 0.05)

0.05 は、大きな標準偏差で係数が 0 や負になるのを防ぐ下限である。絶対のばらつきは公称値に加える。

.. _tbl-mc-perturb:

.. list-table:: モンテカルロ解析のばらつき（標準偏差の既定値）
   :header-rows: 1
   :widths: 22 12 14 52
   :class: longtable

   * - 項目
     - 種類
     - 既定値
     - 反映のしかた
   * - ``thrust_scale``
     - 相対
     - 0.03
     - 推力に :math:`k=f(\epsilon)` を掛ける
   * - ``burn_time_scale``
     - 相対
     - 0.02
     - 推力曲線の時間軸を :math:`\tau=f(\epsilon)` 倍する（後述）
   * - ``dry_mass``
     - 相対
     - 0.1
     - 乾燥質量 :math:`m_d`、:math:`I_{xx}`、:math:`I_{yy}` に :math:`f(\epsilon)` を掛ける
   * - ``cg``
     - 絶対 [m]
     - 0.01
     - 乾燥重心 :math:`x_{cg,d}` に :math:`\epsilon` を足す（正が後方）
   * - ``cn_scale``
     - 相対
     - 0.10
     - 係数表の引き値 :math:`C_N`、:math:`C_{N\alpha}`、ピッチ減衰の和 :math:`S_0,S_1,S_2` に :math:`f(\epsilon)` を掛ける
   * - ``ca_scale``
     - 相対
     - 0.15
     - 係数表の引き値 :math:`C_A^{\mathrm{on}}`、:math:`C_A^{\mathrm{off}}` に :math:`f(\epsilon)` を掛ける
   * - ``elevation_deg``
     - 絶対 [deg]
     - 0.5
     - 射角に :math:`\epsilon` を足す
   * - ``azimuth_deg``
     - 絶対 [deg]
     - 1.0
     - 射方位に :math:`\epsilon` を足す
   * - ``wind_speed``
     - 絶対 [m/s]
     - 1.0
     - 基準風速を :math:`\max(w_{\mathrm{ref}}+\epsilon,\,0)` にする
   * - ``wind_direction_deg``
     - 絶対 [deg]
     - 15.0
     - 基準風向を :math:`(\psi_{\mathrm{ref}}+\epsilon)\bmod360^\circ` にする
   * - ``parachute_cd_s_scale``
     - 相対
     - 0.1
     - パラシュートの :math:`C_DS` に :math:`f(\epsilon)` を掛ける

燃焼時間と推力
^^^^^^^^^^^^^^

推力曲線 :math:`F(t)` を、推力の倍率 :math:`k` と時間軸の倍率 :math:`\tau` で

.. math::
   :label: eq-mc-thrust

   F'(t)=\frac{k}{\tau}\,F\!\left(\frac{t}{\tau}\right)

に置き換える。曲線の各点 :math:`(t_i,F_i)` は :math:`(\tau t_i,\ kF_i/\tau)` に移る。
燃焼時間は :math:`\tau` 倍になるが、時間軸を伸ばした分だけ推力を下げるので、全力積は :math:`\tau` によらず :math:`k` 倍になる:

.. math::

   \int F'\,\mathrm{d}t = k\int F(u)\,\mathrm{d}u .

推進剤の質量は変えない（質量の減り方は力積に比例するので、燃焼の時間経過だけが変わる）。
``thrust_scale`` は全力積を変え、``burn_time_scale`` は力積を保ったまま燃焼時間を変える。

空力係数
^^^^^^^^

空力のばらつきは、係数表の **引いたあとの値** に掛ける。\ :eq:`eq-aero-force` の :math:`C_N` に加え、
:math:`C_{N\alpha}` とピッチ減衰の和 :math:`S_0,S_1,S_2`\ （:eq:`eq-moment-damp`）も同じ倍率 :math:`f_{N}` で変えるので、
圧力中心 :math:`x_{cp}` は変わらず、復元モーメントとピッチ減衰がともに :math:`f_N` 倍になる。
軸力は :math:`C_A^{\mathrm{on}}` と :math:`C_A^{\mathrm{off}}` の両方を :math:`f_A` 倍する。
係数表そのものは全サンプルで共有するので、サンプルごとに作り直すことはない。

風
^^

``wind_speed`` と ``wind_direction_deg`` は、``[wind]`` の基準風速・基準風向の周りのばらつきである。
どちらかの値が 0 でないときだけ、風を作り直す（:ref:`sec-wind-scaling`）。高度別の表（``profile``）では、
表全体が拡大・回転するので、基準高度の風が新しい値になる。

失敗の扱い
~~~~~~~~~~

ばらつきによっては、飛翔計算が正常に終わらないことがある。例えば推力が小さすぎてランチャを離れない場合は、
「ランチャを離れなかった」というエラーになる。また、``max_time`` までに着地しなかった場合も失敗とする。
失敗したサンプルは ``dispersion_mc.csv`` に ``status = failed`` の行として残るが（結果の列は空欄）、
**すべての統計から除かれ**、``failed`` として数える。ほかのサンプルには影響しない。
弾道落下とパラシュート降下は別々に判定する。実行の最後に、失敗したサンプルの理由を最大 3 件まで表示する。

統計量
~~~~~~

降下モードごとに、成功したサンプルの着地点 :math:`\bm{p}_i=(x_{E,i},y_{N,i})`\ （射点からの東・北 [m]）から、次の統計量を求める。
有効なサンプル数を :math:`n` とする。

.. math::
   :label: eq-mc-stat

   \bar{\bm{p}}=\frac1n\sum_i\bm{p}_i,\qquad
   \Sigma=\frac1{n-1}\sum_i(\bm{p}_i-\bar{\bm{p}})(\bm{p}_i-\bar{\bm{p}})^\top
   =\begin{pmatrix}a&b\\ b&c\end{pmatrix}

分母は :math:`n-1`\ （不偏推定）である（:math:`n=1` のときは共分散を 0、:math:`n=0` のときはすべて NaN とする）。
共分散行列 :math:`\Sigma` の固有値は

.. math::
   :label: eq-mc-eigen

   \lambda_{1,2}=\frac{a+c}2\pm\sqrt{\left(\frac{a-c}2\right)^2+b^2}\qquad(\lambda_1\ge\lambda_2\ge0)

で、固有ベクトルが楕円の主軸の向きを与える :cite:`johnson`。:math:`k` シグマの **誤差楕円** は、平均 :math:`\bar{\bm{p}}` を中心として
長半径 :math:`k\sqrt{\lambda_1}`、短半径 :math:`k\sqrt{\lambda_2}` を持つ（:numref:`fig-error-ellipse`、``ellipse_1sigma`` は :math:`k=1`、``ellipse_3sigma`` は :math:`k=3`）。
長軸の向きは、東から反時計回りの角 :math:`\theta=\tfrac12\operatorname{atan2}(2b,\ a-c)` から、北から **時計回り** の方位

.. math::
   :label: eq-mc-bearing

   \beta=(90^\circ-\theta)\bmod180^\circ\quad\in[0^\circ,180^\circ)

として出力する（楕円は点対称なので、向きは 180° の不定性を持つ）。

.. _fig-error-ellipse:

.. figure:: _generated/tikz/error_ellipse.*
   :width: 75%

   着地点の分布（点）と誤差楕円。実線が :math:`1\sigma`、破線が :math:`3\sigma` で、
   長軸の方位 :math:`\beta` は北から時計回りに測る。:math:`\lambda_1,\lambda_2` は共分散行列の固有値である。

二次元の正規分布では、:math:`1\sigma` 楕円の内側に入る確率は :math:`1-e^{-1/2}\approx39\ \%`、:math:`3\sigma` 楕円では :math:`1-e^{-9/2}\approx99\ \%` である :cite:`johnson`。
一次元の「:math:`1\sigma` で 68 %」とは異なるので注意してほしい。
この統計量は、着地点が（二次元の）正規分布に従うという仮定の下での要約である。実際の分布は歪むことがあるので、
最大着地距離（``max_landing_distance``）とそのサンプル番号も併せて出力する。頂点高度の平均と標準偏差（:math:`n-1` で割る）も求める。

出力ファイル
~~~~~~~~~~~~

``dispersion_mc.csv``
   サンプル番号 ``sample``、降下モード ``descent``、``status``、引いたばらつき（``thrust_scale`` から ``parachute_cd_s_scale`` まで。
   相対のものは :math:`\epsilon`\ （割合）、``elevation_deg`` と ``azimuth_deg`` と ``cg`` は加えた量、
   風は ``wind_speed_delta`` [m/s] と ``wind_direction_deg_delta`` [deg]）、そのサンプルの基準風速 ``wind_speed`` と基準風向 ``wind_direction_deg``、
   着地点 ``landing_east`` ``landing_north`` ``landing_lat`` ``landing_lon``、``landing_distance``、``landing_time``、``landing_speed``、
   ``apogee``、``apogee_time``、``max_mach``、``rail_exit_speed``、``min_stability_cal`` の順に並ぶ。

``dispersion_summary.json``
   ``samples``、``seed`` と、降下モードごとの ``descents`` の配列。各要素に ``descent``、``failed``、``n``、``mean_east``、``mean_north``、
   ``covariance``\ （:math:`[[a,b],[b,c]]`）、``ellipse_1sigma`` と ``ellipse_3sigma``\ （``semi_major``、``semi_minor``、``major_axis_bearing_deg``）、
   ``max_landing_distance``、``max_landing_distance_sample``、``apogee_mean``、``apogee_std`` を持つ。

結果の例
~~~~~~~~

サンプル（射角 85°、射方位 270°、北風 4 m/s、``rk45``）に既定のばらつきを与えて 1000 サンプルを計算した結果を :numref:`fig-dispersion-mc` と :numref:`tbl-mc-result` に示す。
計算時間は 1000 サンプル × 2 降下モードで約 0.8 s である（20 スレッドの CPU）。
この図と表は、ドキュメントのビルド時に同じ設定で生成する（``make data``）。

.. _fig-dispersion-mc:

.. figure:: _generated/plots/dispersion_mc.*
   :width: 100%

   モンテカルロ解析による着地点の分布。点が各サンプルの着地点、十字が平均、実線と破線が :math:`1\sigma`、:math:`3\sigma` の誤差楕円、星が射点。
   左が弾道落下、右がパラシュート降下（失敗した 1 サンプルを除く 999 点）。

.. _tbl-mc-result:

.. list-table:: モンテカルロ解析の結果（サンプル、1000 サンプル、``seed = 1``）
   :header-rows: 1
   :widths: 40 30 30

   * - 量
     - 弾道落下
     - パラシュート降下
   * - 有効サンプル数 / 失敗
     - 1000 / 0
     - 999 / 1
   * - 着地点の平均 東 / 北 [m]
     - −1079.7 / 370.8
     - −650.5 / −7653.7
   * - :math:`1\sigma` 楕円 長半径 × 短半径 [m]
     - 226.6 × 127.4
     - 2162.5 × 2067.8
   * - :math:`3\sigma` 楕円 長半径 × 短半径 [m]
     - 679.9 × 382.2
     - 6487.4 × 6203.4
   * - 長軸の方位 [deg]
     - 122
     - 18
   * - 最大着地距離 [m]（サンプル番号）
     - 2057.2（835）
     - 15196.3（935）
   * - 頂点高度の平均 ± 標準偏差 [m]
     - 4480.4 ± 487.8
     - 4477.7 ± 480.2

弾道落下の分布は、射点の西から北西に向かって細長く広がる（長軸の方位は約 122°）。
パラシュート降下は、降下中に風で長く流されるので、主に風速・風向のばらつきが着地点の広がりになる。
広がりは約 2 km（:math:`1\sigma`）で、弾道落下より 1 桁大きい。長半径と短半径がほぼ等しい（2162 m と 2068 m）ので、長軸の方位はあまり意味を持たない。
実際の分布が正規分布から少しずれていることは、:math:`1\sigma`、:math:`3\sigma` 楕円の内側に入る点の割合
（弾道落下で 43 %、98 %、パラシュート降下で 41 %、99 %）が正規分布の 39 %、99 % とやや違うことにも表れている。
頂点高度の平均（4480 m）は、公称の飛翔（4441 m）より 40 m ほど高い。ばらつきの分布は平均について対称でも、頂点高度は推力や抗力係数に対して線形ではないので、平均は公称値と一致しない。

失敗した 1 サンプルは、パラシュート降下の 830 番で、``max_time`` までに着地しなかったものである
（同じサンプルの弾道落下は正常に着地している）。失敗は統計から除かれ、表の有効サンプル数に反映される。

Google Earth での確認
---------------------

落下分散の結果は、衛星写真や地図の上に重ねて見ると、保安域との関係が分かりやすい。
``[output]`` の ``kml`` が ``true``\ （既定）のとき、``ignisyeet dispersion`` は ``ignisyeet.kml`` を書き出す。
このファイルには射点と落下分散のほか、同じ出力先ディレクトリに ``sim`` の結果があれば飛翔経路もフォルダ ``Flight`` として入るので、落下分散と飛翔経路を同じ画面で見られる。
これを Google Earth で開く（ファイルをウィンドウにドラッグするか、「ファイル」メニューの「開く」で選ぶ）と、図形が射点を中心に地形の上へ表示される。
形式は OGC の KML 2.2 :cite:`ogckml` である。

フォルダ ``Dispersion – wind grid``\ （``mode = "wind_grid"``）
   降下モード（弾道落下とパラシュート降下）ごとのフォルダの下に風速ごとのフォルダがあり、チェックボックスで表示を切り替えられる。
   風速ごとに、着地点を風向の順に結んだ輪郭（:numref:`fig-dispersion` の多角形に当たる）と、各ケースの着地点の印が入る。
   印をクリックすると、風速・風向、着地点の東・北方向の距離、着地時刻と着地速度、頂点高度が見られる。

フォルダ ``Dispersion – Monte Carlo``\ （``mode = "monte_carlo"``）
   降下モードごとに、平均着地点、:math:`1\sigma` と :math:`3\sigma` の誤差楕円（:numref:`fig-error-ellipse`）、各サンプルの着地点が入る。
   サンプル数が多いときは、フォルダ ``Landing points`` の表示を切ると楕円だけを見られる。失敗したサンプルは含まれない。

図形はすべて地面に固定して描く。KML には射点の標高を含む高度が入るが、落下分散の図形に限れば高度の扱いは結果に影響しない。
フォルダ ``Flight`` の飛翔経路の色分けや高度の扱い（ジオイド高を扱わないこと）は :ref:`sec-kml` にまとめた。
着地点の位置は、これまでの章と同じく地形の起伏を考えない射点標高の面で決めている（:doc:`limitations`）。
