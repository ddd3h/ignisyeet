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
``constant``、``power``、``log`` では、格子の風速は ``[wind]`` の基準高さ ``ref_height`` での風速である。べき法則と対数則では、この風速を上空へ外挿するので、頂点が地上 1000 m を超えると警告を出す（:ref:`sec-config-checks`）。
高度別の表（``profile``）では、格子の風速は **表の平均風速** :math:`W`\ （最下行から最上行までの高度範囲での平均）で、格子の風向は表の平均風の風向 :math:`\Psi` である。
表は、形（高度ごとの風速比と風向のねじれ）を保ったまま、:math:`k=v/W` 倍に拡大し、:math:`\psi-\Psi` だけ回転する（:ref:`sec-wind-scaling`）。
地上 2 m の風速ではないので、地上風の測定値を ``wind_speeds`` にそのまま入れてはならない。
:math:`W` は表の全範囲での平均なので、表を飛翔の高度範囲（頂点より少し上まで）に切ると、``wind_speeds`` や標準偏差の解釈が容易になる（:ref:`sec-wind-profile`）。:math:`W` は ``ignisyeet sim`` の ``summary.json`` の ``wind_speed`` に出る。
平均風速がほぼ 0 の表はこの拡大ができないので、エラーにする。

.. _sec-wind-grid-failed:

着地しなかったケース
^^^^^^^^^^^^^^^^^^^^

``sim.max_time`` までに着地しないケースは、着地点がなく、失敗として数える。
``dispersion.csv`` には、このケースも ``landed = false`` の行として残り、着地に関する列は ``NaN`` になる。
着地しなかったケースは、着地距離の最大値の計算と、KML の輪郭と着地点から除く。したがって、失敗があるときの最大着地距離は **下限** にすぎない。
端末には、降下モードごとに ``max landing distance ≥ … m (N of M cases did not land: lower bound)`` の形で表示し、どのケースも着地しなければ ``NO CASE LANDED`` と表示して、距離は出さない。
警告に失敗の数を書き、``dispersion_meta.json`` の ``failed`` に数を記録し、終了ステータス 3 で終わる（:ref:`sec-exit-status`）。

積分がエラーで止まったケース（数値の発散など）は、計算全体を止めずに、失敗として記録する。
``dispersion.csv`` のそのケースの行は ``landed = false`` で着地の列が ``NaN`` になり、列 ``error`` にエラーの文を書く（正常なケースでは空）。
``dispersion_meta.json`` の ``errored`` にこのケースの数を記録する（``failed`` はこれを含む）。
警告には、最初の 3 ケースのエラーを風速・風向・降下モードとともに書き、残りは数だけを書く。
エラーで止まったケースは、着地しなかったケースとは別に数え、警告の文も別である（``N of M wind-grid cases failed with an error (e.g. …): they are excluded from the landing distances, so the maximum landing distance is only a lower bound``。着地しなかったケースの文は ``N of M wind-grid cases did not land within sim.max_time = … s: …``）。
終了ステータスは 3 である。
すべてのケースがエラーで止まったときは、``dispersion.csv`` と ``dispersion_meta.json`` を書いたうえで、``all N wind-grid cases failed with an error (first: …); the results are in …`` というエラーで終了ステータス 1 になる。

静的に不安定なケースは、結果から除かずに数えて警告する（:ref:`sec-dispersion-unstable`）。

エラーで止まったケースは、``dispersion_meta.json`` の ``aero_table_range``\ （係数表の範囲外の引き出しの集計）と、その警告にも数えない。
これらの集計は、完了したケースを同じ設定で飛ばし直して求めるので、エラーで止まったケースが途中まで進んだ分の引き出しは入らない。
モンテカルロ法でも、エラーで止まったサンプルがあるときは、完了したサンプルだけを飛ばし直して集計する。

.. _sec-wind-grid-result:

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

多角形（と KML の輪郭）は、隣り合う方位の着地点を直線で結んだ近似である。方位の間隔が粗いと、線の外側にも着地点が出る。
サンプルの風速 7 m/s で方位を 8 から 72 に増やすと、8 方位の多角形の外側に着地点が出る。その多角形からの距離は最大で、パラシュート降下が約 1050 m（風向 70° と 110°）、弾道落下が約 49 m（風向 65° と 115°）である（射方位 270° に対して東西の軸で対称なので、同じ値が 2 方位に出る）。
サンプルの最大着地距離は、8、36、72 方位で同じである（パラシュート降下 14314.1 m、弾道落下 1550.7 m。どちらも最大が東西の風向にあり、どの方位数にも含まれるため）。
ただし最大着地距離は計算した風速と風向の中の最大値なので、一般には方位数と風速の刻みで変わる（パネル法の弾道落下の例は :ref:`sec-dispersion-safety`）。
保安域の判断には、多角形ではなく最大着地距離を半径とする円か、方位を 36 以上にした外側の包絡を使い、格子を細かくして最大値を確かめる（:ref:`sec-dispersion-safety`）。

.. _sec-mc:

モンテカルロ解析（monte_carlo）
-------------------------------

実際の落下点は、風だけでなく推力のばらつき、機体の質量、空力係数の推算誤差、ランチャの設置誤差など多くの要因でばらつく。
モンテカルロ解析は、これらを確率変数として多数回の飛翔計算を行い、着地点の分布を統計量で表す :cite:`metropolis`。
サンプル数を :math:`N`\ （``samples``）として、サンプル :math:`i=0,\dots,N-1` ごとに次の手順を行う。

1. 21 個の標準正規乱数を引き、標準偏差を掛けてばらつきにする（:numref:`tbl-mc-perturb` の 17 項目、機体の非対称な量は 2 軸分、レールの傾きは面内と面に垂直な 2 方向分を引く）。
2. ばらつきを機体・推力・風・回収系に反映した飛翔計算を、弾道落下と（パラシュートが有効なら）パラシュート降下の両方について、**同じ乱数で** 行う。
3. 着地点などの結果を記録する。

サンプルは互いに独立なので、並列に計算する。結果は計算の順序によらず、出力はサンプル番号順（同じ番号では弾道落下が先）に並ぶ。

乱数
~~~~

サンプル :math:`i` の乱数生成器は、ChaCha8 :cite:`bernstein` を ``seed + i``\ （64 ビット整数の足し算で、あふれたら 0 に戻る）で初期化したものである。
したがって、サンプル :math:`i` の値はほかのサンプルの数や実行する順序、スレッド数に依存せず、同じ設定なら **毎回同じ結果** になる。
種は ``seed + i`` だけで決まるので、``seed`` が :math:`s` のサンプル :math:`i` と、``seed`` が :math:`s-1` のサンプル :math:`i+1` は同じ値を引く。

引く値は、次の 21 個の標準正規乱数 :math:`z_1,\dots,z_{21}`\ （:math:`z_k\sim N(0,1)`）に標準偏差 :math:`\sigma_k` を掛けたもの
:math:`\epsilon_k=\sigma_kz_k` である。標準正規乱数は Ziggurat 法 :cite:`marsaglia` で生成する。引く順序は :numref:`tbl-mc-perturb` の上から下の順で、
``thrust_misalignment_deg``, ``thrust_offset``, ``cg_lateral`` はそれぞれ 2 個（:math:`y_b` 方向と :math:`z_b` 方向の順）を引く。
``elevation_deg`` はレールの面内の傾きに 1 個を引き、レールの面に垂直な傾きの 1 個（標準偏差は同じ ``elevation_deg``）は、最後（21 番目）に引く。
**標準偏差が 0 のものも含めて常に 21 個すべてを引く**。そのため、ある項目の標準偏差を 0 にしても（あるいは 0 でなくしても）、
ほかの項目の値は変わらない。1 項目だけの影響を調べるときに、サンプルどうしを比較できる。
表の上から 11 項目（``thrust_scale`` から ``parachute_cd_s_scale``）の乱数は先頭の 11 個で、
ロールと非対称の 6 項目（9 個）はその後ろ、レールの面に垂直な傾きは最後に引く。したがって、ロールと非対称の標準偏差を変えても、最初の 11 項目の値は変わらず、同じ ``seed`` で同じ値を引く。

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
     - 乾燥質量 :math:`m_d` と乾燥時の慣性テンソル（:math:`I_{xx}`、:math:`I_{yy}`、:math:`I_{zz}` と慣性乗積）に :math:`f(\epsilon)` を掛ける
   * - ``cg``
     - 絶対 [m]
     - 0.01
     - 乾燥重心 :math:`x_{cg,d}` に :math:`\epsilon` を足す（正が後方）
   * - ``cn_scale``
     - 相対
     - 0.10
     - 係数表の引き値 :math:`C_N`、:math:`C_{N\alpha}`、ピッチ減衰の和 :math:`S_0,S_1,S_2` に :math:`f(\epsilon)` を掛ける（一般表では :math:`C_Y,C_Z,C_m,C_n` とピッチ・ヨーの和）
   * - ``ca_scale``
     - 相対
     - 0.15
     - 係数表の引き値 :math:`C_A^{\mathrm{on}}`、:math:`C_A^{\mathrm{off}}` に :math:`f(\epsilon)` を掛ける
   * - ``elevation_deg``
     - 絶対 [deg]
     - 0.5
     - レールの向きの設置誤差 :math:`\sigma_{\mathrm{tilt}}`。公称の軸を、面内と面に垂直な 2 方向の独立な傾きで等方に傾ける（後述）
   * - ``azimuth_deg``
     - 絶対 [deg]
     - 1.0
     - 射方位の設定誤差。傾けたレールを鉛直軸の周りに :math:`\epsilon` だけ回す（後述）
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
   * - ``thrust_misalignment_deg``
     - 絶対 [deg]
     - 0
     - 推力の傾きのベクトルに、:math:`y_b` 方向と :math:`z_b` 方向の独立な 2 個の :math:`\epsilon` を足す（後述）
   * - ``thrust_offset``
     - 絶対 [m]
     - 0
     - ノズル出口の点 :math:`(o_y,o_z)` に、:math:`y_b` と :math:`z_b` の独立な :math:`\epsilon` を足す
   * - ``fin_cant_deg``
     - 絶対 [deg]
     - 0
     - 設定したフィンのカントに :math:`\epsilon` を足す（後述）
   * - ``cg_lateral``
     - 絶対 [m]
     - 0
     - 乾燥重心の横ずれ :math:`(y_d,z_d)` に、:math:`y_b` と :math:`z_b` の独立な :math:`\epsilon` を足す
   * - ``cl_scale``
     - 相対
     - 0
     - ロールのモーメント係数 :math:`C_l` とカントの傾き :math:`C_{l\delta}` に :math:`f(\epsilon)` を掛ける
   * - ``clp_scale``
     - 相対
     - 0
     - ロール減衰微係数 :math:`C_{lp}` に :math:`f(\epsilon)` を掛ける

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
``cl_scale`` は :math:`C_l` と :math:`C_{l\delta}` を、``clp_scale`` は :math:`C_{lp}` を同様に倍率で変える。
ロールの係数の倍率は :math:`C_l^{\mathrm{tot}}`\ （:eq:`eq-cl-total`）の強制項と減衰項に別々に効くので、
定常なロール角速度（:eq:`eq-roll-steady`）は :math:`f_{Cl}/f_{Clp}` 倍になる。

.. _sec-mc-rail:

レールの向き
^^^^^^^^^^^^

``elevation_deg`` は、射角だけのばらつきではなく、レールの向きの **等方な設置誤差** の標準偏差 :math:`\sigma_{\mathrm{tilt}}` である。
公称の射角 :math:`\theta_e`、射方位 :math:`\psi` のレールの向き :math:`\bm{d}_0`\ （:eq:`eq-rail-dir`）に対して、
射角が増える向き :math:`\bm{u}_e` と、射方位が増える向き :math:`\bm{u}_c`\ （東・北・上の成分）を

.. math::
   :label: eq-mc-rail-basis

   \bm{u}_e=(-\sin\theta_e\sin\psi,\ -\sin\theta_e\cos\psi,\ \cos\theta_e),\qquad
   \bm{u}_c=(\cos\psi,\ -\sin\psi,\ 0)

とする（:math:`\bm{d}_0,\bm{u}_e,\bm{u}_c` は互いに直交する単位ベクトル）。面内の傾き :math:`\epsilon_e` と面に垂直な傾き :math:`\epsilon_c` を、どちらも :math:`N(0,\sigma_{\mathrm{tilt}}^2)` から独立に引き、
:math:`\vartheta=\sqrt{\epsilon_e^2+\epsilon_c^2}` として、軸を :math:`\vartheta` だけ傾ける。

.. math::
   :label: eq-mc-rail-tilt

   \bm{d}=\cos\vartheta\,\bm{d}_0+\sin\vartheta\,\frac{\epsilon_e\bm{u}_e+\epsilon_c\bm{u}_c}{\vartheta}

公称の軸から測った傾きの角度は :math:`\vartheta` で、その向きは公称の軸のまわりに等方に分布する。
:math:`\vartheta` は 2 次元正規分布の動径なので、Rayleigh 分布（尺度 :math:`\sigma_{\mathrm{tilt}}`、平均 :math:`1.25\,\sigma_{\mathrm{tilt}}`）に従う。
鉛直のレール（:math:`\theta_e=90^\circ`）でも、傾きは全方位に同じ広がりを持つ。このとき面内の方向は公称の射方位 :math:`\psi` の向きである。

``azimuth_deg`` は **方位の設定誤差** で、傾けたレールを鉛直軸のまわりに :math:`\epsilon_\psi\sim N(0,\sigma_{\mathrm{az}}^2)` だけ回す。
軸が水平方向に動く大きさは :math:`\sigma_{\mathrm{az}}\cos\theta_e` で、鉛直のレールには効かない。
飛ばすレールの射角 :math:`\theta_e'=\arcsin d_U` と射方位 :math:`\psi'=\operatorname{atan2}(d_E,\,d_N)+\epsilon_\psi` は、傾きが小さいとき

.. math::

   \theta_e'\approx\theta_e+\epsilon_e,\qquad
   \psi'\approx\psi+\frac{\epsilon_c}{\cos\theta_e}+\epsilon_\psi

になる。面に垂直な傾きは、射角が高いほど方位の変化として大きく現れる（サンプルの 85° では :math:`1/\cos85^\circ\approx11.5` 倍）。
傾きと鉛直軸まわりの回転は、レールだけでなく **機体全体** を回す。機体座標の 3 軸はレールとともに回るので、どの軸も公称の向きから回転角以上には動かない。
回したあとの姿勢を、射角 :math:`\theta_e'`、射方位 :math:`\psi'` と、その射角・射方位でのロール角（``launch.roll_deg`` と同じ定義）で表し直すので、ロール角も変わる。
例えば、鉛直のレールをわずかに傾けて射方位が 180° 変わるときは、ロール角も 180° 変わり、機体の向きはほとんど変わらない。
``dispersion_mc.csv`` の ``elevation_deg``、``azimuth_deg``、``roll_deg`` は、引いた値そのものではなく、実際に飛ばした射角・射方位・ロール角の公称値からの変化 :math:`\theta_e'-\theta_e`、:math:`\psi'-\psi` とロール角の変化（方位とロール角は :math:`(-180^\circ,180^\circ]` に折り返す）である。
このため、:numref:`tbl-mc-result` の例では、射角の変化の標準偏差は 0.5° に近く（0.50°）、射方位の変化の標準偏差は約 5.7°、ロール角の変化の標準偏差は約 5.5° になる。

傾けたレールの射角が :math:`(0,90]` deg の外になるサンプルは、飛ばさずに失敗として数える（:ref:`sec-mc-failure`）。
丸めで 90° を超えるときは 90° にする。公称の射角が低く、:math:`\sigma_{\mathrm{tilt}}` が大きいと、射角が 0° 以下になるサンプルが出る。

ロールと非対称のばらつき
^^^^^^^^^^^^^^^^^^^^^^^^

``thrust_misalignment_deg``、``thrust_offset``、``fin_cant_deg``、``cg_lateral``、``cl_scale``、``clp_scale`` の標準偏差の既定値は 0 で、
指定したときだけ有効になる（0 のときは公称値のままである）。

* **推力の傾き**：設定した傾き :math:`(\varepsilon,\phi)`\ （:eq:`eq-thrust-dir`）を、ベクトル :math:`(\varepsilon\cos\phi,\ \varepsilon\sin\phi)`\ [deg] で表し、
  引いた 2 個の値を :math:`y_b` 成分と :math:`z_b` 成分に足す。新しい傾きの角度は足したベクトルの大きさ、方位は :math:`\operatorname{atan2}` で求める。
  公称値が 0 でも、2 つの正規乱数から傾きの角度と方位が決まる。
* **ノズルのオフセット**：設定したノズル出口の点に、軸ごとに足す。
* **フィンのカント**：設定したカント :math:`\delta` に :math:`\epsilon` を足す。係数表の :math:`\delta_{\mathrm{ref}}` は変えないので、
  差の項 :math:`C_{l\delta}(\delta-\delta_{\mathrm{ref}})`\ （:eq:`eq-cl-total`）の分だけロールの強制が変わる。後ろ向きの飛行では、この項も基準のカントの項と同じく向きが反転する（:ref:`sec-roll-moment`）。カントが変えるのはロールのモーメントだけで、:math:`C_N` などは変わらない。
* **乾燥重心の横ずれ**：設定した ``cg_dry`` の横ずれ ``cg_lateral_dry`` に、軸ごとに足す。0 でない横ずれがあると機体は軸対称でなくなり、慣性テンソルの行列を使って計算する（:ref:`sec-inertia-tensor`）。

風
^^

``wind_speed`` と ``wind_direction_deg`` は、``[wind]`` の基準風速・基準風向の周りのばらつきである。
どちらかの値が 0 でないときだけ、風を作り直す（:ref:`sec-wind-scaling`）。高度別の表（``profile``）では、
``wind_speed`` [m/s] は表の平均風速 :math:`W` に加える量で、表全体を :math:`k=\max(W+\delta,0)/W` 倍に拡大する。
``wind_direction_deg`` は表の平均風の風向 :math:`\Psi` に加える角度で、表全体をその角度だけ回転する。
:math:`W` が 0.1 m/s 未満の表で ``wind_speed`` または ``wind_direction_deg`` の標準偏差が 0 でないときは、エラーにする。

.. _sec-mc-failure:

失敗の扱い
~~~~~~~~~~

ばらつきによっては、飛翔計算が正常に終わらないことがある。例えば推力が小さすぎてランチャを離れない場合は、
「ランチャを離れなかった」というエラーになる。傾けたレールの射角が範囲外のサンプルも失敗で、``perturbed rail elevation -0.412 deg is outside (0, 90]`` の形（数は例）のエラーを記録する。
また、``sim.max_time`` までに着地しなかった場合も失敗とする。
失敗したサンプルは ``dispersion_mc.csv`` に ``status = failed`` の行として残るが（結果の列は空欄）、
**すべての統計から除かれ**、``failed`` として数える。着地しなかったサンプルの数は、``failed`` の内数として ``not_landed`` に別に数える。ほかのサンプルには影響しない。
弾道落下とパラシュート降下は別々に判定する。実行の最後に、失敗したサンプルの理由を最大 3 件まで表示する。

失敗したサンプルがあるとき、統計は着地したサンプルだけを表し、真の広がりは報告される値より大きいか不明である。
そのため、その降下モードの ``complete`` を ``false`` にし、最大着地距離を下限として ``max_landing_distance_is_lower_bound`` を ``true`` にする。
端末には ``STATISTICS INCOMPLETE`` と最大着地距離の下限（``≥``）を表示し、警告に ``INCOMPLETE`` の文を加える。
どのサンプルも着地しなければ、統計と距離は ``null`` になり、``no sample landed`` と表示する。
失敗が 1 件でもあれば、出力を書いたうえで終了ステータス 3 で終わる（:ref:`sec-exit-status`）。
着地したサンプルが一つもなければ、KML にモンテカルロの結果は入れない。着地したサンプルが 1 つだけの降下モードは、平均着地点だけを入れ、誤差楕円は入れない。
着地したサンプルが 2 つ未満の降下モードでは、端末の誤差楕円の行に ``n/a (needs at least 2 landed samples)`` と表示し、頂点高度の標準偏差も ``n/a`` とする。
着地したサンプルが 30 未満の降下モードには、``Ballistic: only 12 landed samples: the statistics are rough`` の形（降下モード名と数は例）の警告を出す。

.. _sec-dispersion-unstable:

静的に不安定なケース
~~~~~~~~~~~~~~~~~~~~

風の格子のケースとモンテカルロのサンプルは、推力・質量・重心・空力係数などが公称値から外れるので、公称の飛翔が安定でも、一部のケースが静的に不安定になりうる。
静安定余裕の最小値（自由飛行中の最小値とランチャ離脱時の値の小さいほう）が 0 cal 未満の飛翔を、静的に不安定とする。
このようなケースとサンプルは、降下モードごとに数える。統計と最大着地距離には **入れたまま** にする。不安定な機体は転倒して飛ぶので、着地点が大きく変わりうる。除くと分布の裾を過小に見積もる向きになる。

* 警告に、降下モードごとに ``N of M Monte Carlo samples (ballistic) are statically UNSTABLE (min static margin down to X cal): they are kept in the statistics (conservative), so the landing distances include tumbling flights`` の形の文を書く。
  風の格子では ``Monte Carlo samples`` が ``wind-grid cases`` になる。:math:`M` は、失敗したものを含む、その降下モードのケース（サンプル）の数である。
* 終了ステータス 3 で終わり、標準エラー出力に ``RESULTS NOT USABLE: N of M Monte Carlo samples (ballistic) are statically UNSTABLE (min static margin down to X cal): increase the static margin or reduce the CG/aero sigmas`` の形の行を書く。
  風の格子の末尾は ``increase the static margin`` である。降下モードが複数あるときは、モードごとの節を ``;`` でつなぐ。公称の飛翔が不安定なときは、その文のほうを優先する（:ref:`sec-exit-status`）。
* ``dispersion_meta.json``\ （風の格子）と ``dispersion_summary.json``\ （モンテカルロ）の ``unstable``\ （降下モードごとの数）と ``unstable_min_static_margin_cal``\ （降下モードごとの最小の余裕）に残す。
* ``dispersion.csv`` と ``dispersion_mc.csv`` の最後の列 ``stable`` は、その行の飛翔が静的に安定なら ``true``、不安定なら ``false``、判定できない行と失敗した行では空である。
  ``false`` の行だけを取り出すと、不安定になった条件が分かる。

一般表で、ケースやサンプルの静安定余裕が求まらない（法線力の傾きが 0 か物理的でない符号の面がある。:ref:`sec-static-margin-general`）ときは、そのケースの静的な安定は判定できない。
判定できないケースとサンプルも降下モードごとに数えて統計に残し、警告に ``N of M Monte Carlo samples (ballistic): static stability could not be evaluated (first: …)`` の形の文（括弧内は最初のケースの理由）を書いて、終了ステータス 3 で終わる。
数は ``dispersion_meta.json`` と ``dispersion_summary.json`` の ``stability_unknown_runs`` に残る。

不安定なケースが出たときは、静安定余裕を増やすか（フィンを大きくする、重心を前へ寄せる）、重心や空力係数の標準偏差を見直す。
終了ステータスが 3 の結果は、保安の判断に使わない。

統計量
~~~~~~

降下モードごとに、成功したサンプルの着地点 :math:`\bm{p}_i=(x_{E,i},y_{N,i})`\ （射点からの東・北 [m]）から、次の統計量を求める。
有効なサンプル数を :math:`n` とする。

.. math::
   :label: eq-mc-stat

   \bar{\bm{p}}=\frac1n\sum_i\bm{p}_i,\qquad
   \Sigma=\frac1{n-1}\sum_i(\bm{p}_i-\bar{\bm{p}})(\bm{p}_i-\bar{\bm{p}})^\top
   =\begin{pmatrix}a&b\\ b&c\end{pmatrix}

分母は :math:`n-1`\ （不偏推定）である。平均は :math:`n\ge1`、共分散・標準偏差・誤差楕円は :math:`n\ge2` のときだけ求まり、サンプルが足りない統計は NaN とする（``dispersion_summary.json`` では ``null``）。
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
最大着地距離（``max_landing_distance``）とそのサンプル番号も併せて出力する。頂点高度の平均（:math:`n\ge1`）と標準偏差（:math:`n-1` で割る。:math:`n\ge2`）も求める。

着地距離の分布の裾は、パーセンタイル（P50、P90、P99）でも報告する。着地したサンプルの着地距離を昇順に並べた :math:`d_{(0)}\le\dots\le d_{(n-1)}` から、:math:`q` パーセンタイルを線形補間

.. math::
   :label: eq-mc-percentile

   d_q = d_{(i)} + (d_{(i+1)}-d_{(i)})\,f,\qquad x=\frac{q}{100}(n-1),\quad i=\lfloor x\rfloor,\quad f=x-i

で求める（:math:`n=1` ならその値、:math:`n=0` なら NaN）。
誤差楕円は着地点の分布を二次元の正規分布とみなした要約なので、P90 や P99 は、保安域の半径を考えるときに分布の形によらない目安になる。
パーセンタイルも、失敗したサンプルを含まないので、``complete`` が ``false`` のときは過小になりうる。

出力ファイル
~~~~~~~~~~~~

``dispersion_mc.csv``
   サンプル番号 ``sample``、降下モード ``descent``、``status``、引いたばらつき（``thrust_scale`` から ``parachute_cd_s_scale`` まで。
   相対のものは :math:`\epsilon`\ （割合）、``cg`` は加えた量、``elevation_deg``、``azimuth_deg``、``roll_deg`` は実際に飛ばした射角、射方位、ロール角の公称値からの変化（:ref:`sec-mc-rail`）、
   風は ``wind_speed_delta`` [m/s] と ``wind_direction_deg_delta`` [deg]）、そのサンプルの基準風速 ``wind_speed`` と基準風向 ``wind_direction_deg``、
   着地点 ``landing_east`` ``landing_north`` ``landing_lat`` ``landing_lon``、``landing_distance``、``landing_time``、``landing_speed``、
   ``apogee``、``apogee_time``、``max_mach``、``rail_exit_speed``、``min_stability_cal`` の順に並び、ロールと非対称のばらつき（9 列）が続き、最後の列 ``stable`` が静的に安定かを示す（:ref:`sec-dispersion-unstable`）。

``dispersion_summary.json``
   ``samples``、``seed`` と、降下モードごとの ``descents`` の配列、設定のハッシュ ``config_hash``、警告 ``warnings``。各要素に ``descent``、``failed``、``not_landed``、``complete``、
   ``max_landing_distance_is_lower_bound``、``landing_distance_p50`` ``landing_distance_p90`` ``landing_distance_p99``、``n``、``mean_east``、``mean_north``、
   ``covariance``\ （:math:`[[a,b],[b,c]]`）、``ellipse_1sigma`` と ``ellipse_3sigma``\ （``semi_major``、``semi_minor``、``major_axis_bearing_deg``）、
   ``max_landing_distance``、``max_landing_distance_sample``、``apogee_mean``、``apogee_std`` を持つ。降下モードごとの静的に不安定なサンプルの数 ``unstable`` と最小の静安定余裕 ``unstable_min_static_margin_cal``、判定できないサンプルの数 ``stability_unknown_runs``、
   公称の風（``wind_model``、``wind_speed``、``wind_direction_deg``、``profile`` では ``wind_scale``）もある（:ref:`sec-file-dispersion`）。

``dispersion.csv``\ （``mode = "wind_grid"``）
   ケースごとの結果で、列 ``landed`` が着地したかを、最後の列 ``stable`` が静的に安定かを示す。``dispersion_meta.json`` に設定のハッシュ、ケース数、失敗数、降下モードごとの不安定なケースの数、警告を書く（:ref:`sec-file-dispersion`）。

結果の例
~~~~~~~~

サンプル（射角 85°、射方位 270°、北風 4 m/s、``rk45``）に既定のばらつきを与えて 1000 サンプルを計算した結果を :numref:`fig-dispersion-mc` と :numref:`tbl-mc-result` に示す。
計算時間の目安は :doc:`overview` の表にある。
この図と表は、ドキュメントのビルド時に同じ設定で生成する（``make data``）。設定は ``examples/sample.toml`` の ``[sim] integrator = "rk45"`` と ``[dispersion] mode = "monte_carlo"`` の 2 箇所だけを変えたもので、サンプル数 1000、``seed = 1``、標準偏差はファイルのままである。

.. _fig-dispersion-mc:

.. figure:: _generated/plots/dispersion_mc.*
   :width: 100%

   モンテカルロ解析による着地点の分布。点が各サンプルの着地点、十字が平均、実線と破線が :math:`1\sigma`、:math:`3\sigma` の誤差楕円、星が射点。
   左が弾道落下、右がパラシュート降下（どちらも 1000 点）。

.. _tbl-mc-result:

.. list-table:: モンテカルロ解析の結果（サンプル、1000 サンプル、``seed = 1``）
   :header-rows: 1
   :widths: 40 30 30

   * - 量
     - 弾道落下
     - パラシュート降下
   * - 有効サンプル数 / 失敗
     - 1000 / 0
     - 1000 / 0
   * - 着地点の平均 東 / 北 [m]
     - −1079.4 / 370.3
     - −651.0 / −7663.6
   * - :math:`1\sigma` 楕円 長半径 × 短半径 [m]
     - 237.1 × 149.4
     - 2185.3 × 2066.8
   * - :math:`3\sigma` 楕円 長半径 × 短半径 [m]
     - 711.4 × 448.2
     - 6555.8 × 6200.4
   * - 長軸の方位 [deg]
     - 131
     - 16
   * - 着地距離 P50 / P90 / P99 [m]
     - 1130.8 / 1458.8 / 1848.0
     - 7835.4 / 10907.3 / 13666.3
   * - 最大着地距離 [m]（サンプル番号）
     - 2139.8（935）
     - 18019.9（830）
   * - 頂点高度の平均 ± 標準偏差 [m]
     - 4480.2 ± 487.4
     - 4480.2 ± 487.4

弾道落下の分布は、射点の西から北西に向かってやや細長く広がる（長軸の方位は約 131°）。
パラシュート降下は、降下中に風で長く流されるので、主に風速・風向のばらつきが着地点の広がりになる。
広がりは約 2 km（:math:`1\sigma`）で、弾道落下より 1 桁大きい。長半径と短半径がほぼ等しい（2185 m と 2067 m）ので、長軸の方位はあまり意味を持たない。
実際の分布が正規分布から少しずれていることは、:math:`1\sigma`、:math:`3\sigma` 楕円の内側に入る点の割合
（弾道落下で 44 %、98 %、パラシュート降下で 41 %、99 %）が正規分布の 39 %、99 % とやや違うことにも表れている。
頂点高度の平均（4480 m）は、公称の飛翔（4441 m）より 40 m ほど高い。ばらつきの分布は平均について対称でも、頂点高度は推力や抗力係数に対して線形ではないので、平均は公称値と一致しない。

この計算では、どちらの降下モードも 1000 サンプルがすべて着地したので、``failed`` と ``not_landed`` は 0、``complete`` は ``true`` で、統計は完全である。
着地距離は、弾道落下で P50 が 1131 m、P90 が 1459 m、P99 が 1848 m、パラシュート降下で P50 が 7835 m、P90 が 10907 m、P99 が 13666 m である。
最大着地距離はパラシュート降下の 830 番の 18020 m で、P99 より 4 km ほど遠い。分布の裾が長いので、保安域の半径は誤差楕円よりパーセンタイルや最大値で考えるとよい。
着地しないサンプルがあるときの扱いは、:ref:`sec-mc-failure` に述べた。

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
   サンプル数が多いときは、フォルダ ``Landing points`` の表示を切ると楕円だけを見られる。失敗したサンプルと着地しなかったサンプルは含まれない。

風の格子の輪郭は、隣り合う方位の着地点を直線で結ぶので、方位数が少ないと着地点の一部が輪郭の外に出る（上の「結果の見方」）。保安域の確認には、輪郭だけでなく着地点の印も見る。
風の格子では、着地しなかったケースを輪郭と着地点から除く。KML は、設定のハッシュが現在の設定と一致する結果だけをまとめる（:ref:`sec-kml`）。

図形はすべて地面に固定して描く。KML には射点の標高を含む高度が入るが、落下分散の図形に限れば高度の扱いは結果に影響しない。
フォルダ ``Flight`` の飛翔経路の色分けや高度の扱い（ジオイド高を扱わないこと）は :ref:`sec-kml` にまとめた。
着地点の位置は、ほかの章と同じく地形の起伏を考えない射点標高の面で決めている（:doc:`limitations`）。

.. _sec-dispersion-safety:

保安距離の検討に使う手順
------------------------

打上げの安全を確認するときの、落下分散の使い方を順に示す。数値の基準（保安距離そのものの規定）は、打上げを行う場所と団体の規則に従う。ここでは規定の数値は与えない。

どの計算の最大着地距離も、**計算した条件（風速、風向、サンプル）の中での最大値** である。計算していない条件の着地点は、これより遠くなりうる。
以下の手順は、遠い着地点を取りこぼさないように、条件の組合せを広げていくものである。最後に、すべての最大値の最大に余裕を足して半径とする。

準備
~~~~

手順は、一つの設定ファイルを書き換えずに、コマンドラインの ``--set``、``--out``、``--aero-table``\ （:ref:`sec-cli-overrides`）で条件を変えて実行する。
コマンドは **bash** で実行する。``--set`` の組は bash の配列に入れ、``"${P[@]}"`` の形で展開する。
こうすると、``[7.8, 11.9]`` のように空白や角かっこを含む値も 1 つの引数のまま渡り、シェルの単語分割やファイル名の展開（グロブ）で壊れない。
zsh などのほかのシェルを使っているときは、先に ``bash`` を起動する。
結果は、現在のディレクトリの ``runs/`` の下に計算ごとに分けて書く。

まず、``ignisyeet`` を ``PATH`` に入れ、設定ファイルのパスを ``C`` に入れる。
ソースからビルドしたときは、リポジトリの最上位のディレクトリで次を実行する。
インストーラで入れたときは ``~/.local/bin`` が ``PATH`` に入っていればよく（:ref:`sec-install-path`）、例題をコピーしたディレクトリ（:ref:`sec-install-check`）の設定ファイルを ``C`` にする。
自分の機体では、その設定ファイルを ``C`` にする。

.. code-block:: bash

   export PATH="$PWD/target/release:$PATH"   # ソースからビルドしたとき
   C=examples/sample.toml                    # インストーラなら
                                             # C=ignisyeet-examples/sample.toml

各コマンドの終了ステータスを残すため、``ignisyeet`` を関数 ``run`` で包む。
終了ステータスが 0 でなかったコマンドは、``runs/status.txt`` に書かれる（手順 7 で確かめる）。

.. code-block:: bash

   mkdir -p runs && : > runs/status.txt
   run() {
     ignisyeet "$@" || echo "exit $?: ignisyeet $*" >> runs/status.txt
   }

格子で最も遠くなった風を読む関数 ``worst`` も定義しておく。
引数は出力先と降下モード（``ballistic`` または ``parachute``）である。
その出力先の ``dispersion.csv`` で、その降下モードの着地したケースのうち ``landing_distance`` が最大の行の、``wind_speed`` と ``wind_direction_deg`` を出す。

.. code-block:: bash

   worst() {
     python3 - "$1/dispersion.csv" "$2" <<'EOF'
   import csv, sys
   f, d = sys.argv[1:]
   rows = [r for r in csv.DictReader(open(f))
           if r["landed"] == "true" and r["descent"] == d]
   r = max(rows, key=lambda r: float(r["landing_distance"]))
   print(r["wind_speed"], r["wind_direction_deg"])
   EOF
   }

高度別の風の表は ``wind.profile`` で与える。例の ``examples/wind_profile_example.csv`` は書式を示す架空の表なので、実際には当日の予報や観測の表に置き換える。
``profile`` では ``[wind]`` の ``speed`` と ``direction_deg`` は使われない。表の風を変えるときは、``wind.profile_mean_speed`` と ``wind.profile_mean_direction_deg`` で表全体を拡大・回転する（:ref:`sec-wind-scaling`）。
サンプルの設定ファイルは ``speed`` と ``direction_deg`` を書いているので、以下のコマンドでは、使わないことを知らせる警告（``wind.speed is ignored for wind.model = "profile"; …``）が毎回出る。結果には影響しない。
自分の設定ファイルで ``profile`` を使うときは、``[wind]`` からこの 2 つを消しておく。

すべての計算に共通の ``--set`` を配列 ``P`` に、二段開傘（:ref:`sec-dual-deploy`）の正常な開傘の 2 項目を配列 ``N`` に入れる。
``--set`` は項目を消せないので、二段開傘の機体では ``recovery.main_cd_s`` と ``recovery.main_altitude`` を設定ファイルに書かず（ファイルの ``recovery.cd_s`` はドローグの抗力面積）、手順 0〜4 のすべての計算に ``"${N[@]}"`` で加える。
こうすると、手順 0〜4 は正常に開傘する機体を飛ばし、手順 5 では ``N`` を除く（ドローグだけ）か、``N`` の代わりにメインが頂点で開く設定を入れて、故障を飛ばせる。
単一のパラシュートの機体では ``N=()``\ （空の配列）とする。

.. code-block:: bash

   P=(--set wind.model=profile
      --set wind.profile=wind_profile_example.csv)
   N=()   # 単一のパラシュート

サンプルを二段開傘の機体として試すとき（下の計算例。ドローグ 0.5 m\ :sup:`2`、高度 300 m でメイン 8 m\ :sup:`2`）は、続けて次を実行する。

.. code-block:: bash

   P+=(--set recovery.cd_s=0.5)   # ファイルの cd_s をドローグの値に
   N=(--set recovery.main_cd_s=8 --set recovery.main_altitude=300)

各手順の計算と、使う回収系の設定を :numref:`tbl-safety-steps` にまとめる。

.. _tbl-safety-steps:

.. list-table:: 手順と回収系の設定
   :header-rows: 1
   :widths: 10 46 44

   * - 手順
     - 計算
     - 回収系
   * - 0
     - 係数表と公称の飛翔
     - 正常な開傘（``N``）
   * - 1、2
     - 風の格子（36 方位、細分した格子）
     - 正常な開傘（``N``）
   * - 3
     - 手順 2 の格子で最も遠くなった風での、機体のばらつきのモンテカルロ解析
     - 正常な開傘（``N``）。風は ``N`` の格子の最悪の風
   * - 4
     - 予報の風でのモンテカルロ解析
     - 正常な開傘（``N``）
   * - 5
     - 故障ごとの風の格子と、その格子で最も遠くなった風でのモンテカルロ解析
     - メインが頂点で開く（``A``）、ドローグだけ（``N`` なし）。二段開傘の機体だけ
   * - 6
     - 空力の手法の違い（手順 0〜5 を両方の手法で）
     - 各手順のとおり
   * - 7
     - 終了ステータスと警告の確認、半径の決定
     - ―

手順
~~~~

0. **係数表を作り、表の平均風を確かめる。**
   空力の手法ごとに係数表を一度だけ作り、以後の計算は ``--aero-table`` でそれを使う（手順 6 のため、部品積み上げ法とパネル法の両方を作る）。
   表の平均風速 :math:`W` と平均風の風向 :math:`\Psi`\ （:ref:`sec-wind-profile`）は、``sim`` の ``summary.json`` の ``wind_speed`` と ``wind_direction_deg`` に出る。
   開傘時の対気速度と動圧（手順 5）も、この ``summary.json`` に出る。

   .. code-block:: bash

      for m in barrowman panel; do
        run aero "$C" --set aero.method=$m --out runs/aero_$m
      done
      run sim "$C" "${P[@]}" "${N[@]}" \
          --aero-table runs/aero_barrowman --out runs/nominal

   例の表では :math:`W=11.89` m/s、:math:`\Psi=220.2^\circ` である。

1. **風の格子を、打上げ可能な風速の上限まで、36 方位以上で計算する。**
   ``dispersion.wind_speeds`` は、打上げを中止する風速（ランチ風速の上限）までを含め、``directions`` は 36 以上（10° 間隔以下）にする。
   ``power`` と ``log`` は地上の基準風速を上空へ外挿するので、上空の風が強い日の着地点を過小にも過大にも見積もる（:ref:`sec-wind`）。保安の検討には高度別の表を使う。
   ``wind_speeds`` の意味は風モデルで異なる。

   * ``constant``、``power``、``log``：``wind.ref_height``\ （既定 2 m）での風速である。団体が風速の上限を測る高さ（地上 2 m、10 m など）に合わせて ``ref_height`` を設定し、その高さでの上限までを含める。
   * ``profile``：表の **平均風速** :math:`W` である（地上の風速ではない）。表の形はそのままに、全体を :math:`k=v/W` 倍にする（:ref:`sec-wind-scaling`）。
     団体の上限 :math:`V_{\mathrm{lim}}`\ （測る高さ :math:`h_{\mathrm{lim}}` での風速）を表に当てはめるには、表のその高さの風速 :math:`w(h_{\mathrm{lim}})` を読み、
     上限の風速に当たる格子の風速を :math:`v_{\max}=W\,V_{\mathrm{lim}}/w(h_{\mathrm{lim}})` とする。
     格子には :math:`v=W`\ （表そのまま、:math:`k=1`）と :math:`v_{\max}` までの値を入れる。
     :math:`k` が 3 を超える、または 1/3 を下回る風速があると警告が出る。表の形を保った拡大が現実的でないので、上限の風速を与えた着地点の上空の風速を確かめ（``wind_scale`` と警告に出る上空の風速）、
     ふさわしくなければ、上限の風速ごとに別の表を用意する。

   例として、上限を地上 2 m で 7 m/s とする。例の表の 2 m の風速は 0 m と 10 m の行の補間で :math:`w(2\ \mathrm{m})=3.04` m/s なので、:math:`v_{\max}=11.89\times7/3.04=27.4` m/s である。
   格子には、地上 2 m で 2、4、5、6、7 m/s に当たる 7.8、15.6、19.6、23.5、27.4 m/s と、:math:`W` そのものの 11.9 m/s を入れる（:math:`k=0.66` から 2.3）。
   格子の風速は配列 ``G`` に入れ、手順 2 と 5 でも使う。

   .. code-block:: bash

      G=(--set 'dispersion.wind_speeds=[7.8, 11.9, 15.6, 19.6, 23.5, 27.4]')
      for m in barrowman panel; do
        run dispersion "$C" "${P[@]}" "${N[@]}" "${G[@]}" \
            --set aero.method=$m --aero-table runs/aero_$m \
            --set dispersion.directions=36 --out runs/grid_$m
      done

2. **格子を細かくして、最大値が変わらないことを確かめる。**
   方位を増やした格子（72、360 など）と比べ、さらに最大となったケースの風速と風向の近くを細かくした格子（風速の刻みを小さくし、方位を 360 まで増やす）でも計算する。
   最大値が変わる間は細かくし、最も大きい値を使う。最大は風速の上限で起きるとは限らない。弾道落下では、途中の風速で最も遠くなることがある。
   多角形と KML の輪郭は着地点を直線で結んだ近似で、方位が粗いと着地点が輪郭の外に出る（:ref:`sec-wind-grid-result`）。保安域は、輪郭ではなく最大着地距離の円か、細かい格子の外側の包絡で考える。

   .. code-block:: bash

      for m in barrowman panel; do
        run dispersion "$C" "${P[@]}" "${N[@]}" "${G[@]}" \
            --set aero.method=$m --aero-table runs/aero_$m \
            --set dispersion.directions=360 --out runs/grid360_$m
      done
      worst runs/grid360_panel ballistic   # 例：27.4 4

   最も遠いケースの風速と風向は、``worst`` で降下モードごとに読む（``profile`` では平均風速と平均風の風向）。
   風速の刻みを細かくした格子で最大値が増えたときは、手順 3 ではその格子の出力先を ``worst`` に与える。

3. **機体のばらつきを、最も遠くなる風で飛ばす。**
   降下モードと空力の手法ごとに、手順 2 の格子（正常な開傘の格子）で最も遠くなったケースの風を公称の風にして、``mode = "monte_carlo"`` で計算する。
   回収系は正常な開傘（``N``）である。故障の場合の最も遠い風は故障ごとに違うので、手順 5 で別に求める。
   ``profile`` では、そのケースの風速と風向を ``wind.profile_mean_speed`` と ``wind.profile_mean_direction_deg`` に与える。表はそのケースと同じ倍率と角度で拡大・回転され、公称の飛翔は格子のそのケースと同じ風になる。
   ``[wind]`` の ``speed`` と ``direction_deg`` は ``profile`` では使われないので、これらを変えても表の風は変わらない（``constant``、``power``、``log`` では逆に ``wind.speed`` と ``wind.direction_deg`` に与える）。
   風は格子で調べたので、``wind_speed`` と ``wind_direction_deg`` の標準偏差は 0 にし、推力、質量、空力係数、レールの向きなどの機体のばらつきだけを与える。
   機体のばらつきで最も遠くなる風向がずれることがあるので、上限の風速でほかの風向（たとえば 30° ごと）も計算する。
   風の格子は公称の機体だけを飛ばすので、機体のばらつきと強い風が重なった着地点は、この手順でしか出ない。
   サンプル数は :math:`10^4` 以上にし、最大値がサンプル数で変わることに注意する（下の「サンプル数と最大値」）。
   ばらつきの配列 ``MC`` は手順 5 でも使う。

   .. code-block:: bash

      MC=(--set dispersion.mode=monte_carlo
          --set dispersion.monte_carlo.samples=10000
          --set dispersion.monte_carlo.wind_speed=0
          --set dispersion.monte_carlo.wind_direction_deg=0)
      for m in barrowman panel; do
        for k in ballistic parachute; do
          read -r v d < <(worst runs/grid360_$m $k)
          run dispersion "$C" "${P[@]}" "${N[@]}" "${MC[@]}" \
              --set aero.method=$m --aero-table runs/aero_$m \
              --set wind.profile_mean_speed="$v" \
              --set wind.profile_mean_direction_deg="$d" \
              --out runs/mc_worst_${m}_$k
        done
      done
      # 上限の風速で 30° ごとの風向
      for m in barrowman panel; do
        for d in $(seq 0 30 330); do
          run dispersion "$C" "${P[@]}" "${N[@]}" "${MC[@]}" \
              --set aero.method=$m --aero-table runs/aero_$m \
              --set wind.profile_mean_speed=27.4 \
              --set wind.profile_mean_direction_deg="$d" \
              --out runs/mc_${m}_$d
        done
      done

   ``dispersion_summary.json`` の ``wind_speed``、``wind_direction_deg``、``wind_scale`` で、公称の風が意図した値になったことを確かめる。

4. **予報の風でモンテカルロ解析を行い、標準偏差の根拠を残す。**
   当日の予報の表そのものを公称の風にして、風のばらつき（予報の誤差）も含めて計算する。回収系は正常な開傘（``N``）である。
   ``profile`` の風速の標準偏差は平均風速 :math:`W` の周りの揺らぎで、表全体を拡大する（:ref:`sec-wind-scaling`）。
   標準偏差は利用者が決める値で、既定値は目安にすぎない（:ref:`sec-mc`）。推力は燃焼試験のばらつき、空力係数は手法どうしの差（:ref:`sec-lim-aero`）、風は予報の誤差のように、根拠を決めて設定する。

   .. code-block:: bash

      for m in barrowman panel; do
        run dispersion "$C" "${P[@]}" "${N[@]}" \
            --set aero.method=$m --aero-table runs/aero_$m \
            --set dispersion.mode=monte_carlo \
            --set dispersion.monte_carlo.samples=10000 \
            --out runs/mc_forecast_$m
      done

5. **回収系の故障を飛ばす。**
   弾道落下（パラシュートが開かない場合）は、手順 1〜4 で常に計算されるので、その最大値を含める。
   弾道落下は回収系の設定によらないので、故障の計算でも手順 1〜4 と同じ値になる。
   単一のパラシュートの機体では、この手順はこれだけである。
   二段開傘の機体では、さらに次の 2 つの故障を、手順 2 と同じ格子と、その格子でパラシュート降下が最も遠くなった風でのモンテカルロ解析（手順 3 と同じばらつき）で計算する。

   * **メインが頂点で開く**\ （配列 ``A``）：``main_altitude`` を頂点より高くする（例えば 100000）。メインがドローグと同時に開き、``recovery.main_altitude = … m is above the apogee`` の警告が出る。
   * **メインが開かず、ドローグだけで降りる**：``N`` を加えない（``main_cd_s`` と ``main_altitude`` を指定しない）。

   ``A`` のメインの抗力面積は ``N`` と同じ値にする。

   .. code-block:: bash

      A=(--set recovery.main_cd_s=8 --set recovery.main_altitude=100000)
      for f in apogee drogue; do
        F=()                                  # drogue：ドローグだけ
        if [ "$f" = apogee ]; then F=("${A[@]}"); fi
        for m in barrowman panel; do
          run dispersion "$C" "${P[@]}" "${F[@]}" "${G[@]}" \
              --set aero.method=$m --aero-table runs/aero_$m \
              --set dispersion.directions=360 \
              --out runs/grid360_${f}_$m
          read -r v d < <(worst runs/grid360_${f}_$m parachute)
          run dispersion "$C" "${P[@]}" "${F[@]}" "${MC[@]}" \
              --set aero.method=$m --aero-table runs/aero_$m \
              --set wind.profile_mean_speed="$v" \
              --set wind.profile_mean_direction_deg="$d" \
              --out runs/mc_worst_${f}_$m
        done
      done

   風による流されは、降下時間が長いほど大きい。メインが頂点で開くと降下時間が最も長く、着地点は風下へ最も遠い。
   ドローグだけの降下は、ドローグの抗力面積が小さく降下が速ければ、正常な二段開傘より **近く** に落ちる（着地速度は大きい）。
   ドローグが大きく降下が遅ければ遠くなるので、どちらになるかは降下時間による。思い込みで除かず、計算して大きいほうを取る。
   パラシュートが開くときの対気速度と動圧は ``summary.json`` の ``drogue_deploy_speed`` ``drogue_deploy_q`` ``main_deploy_speed`` ``main_deploy_q`` に出る。
   50 m/s または 2 kPa を超える開傘には警告が出る（:ref:`sec-config-checks`）。着地点はパラシュートが壊れないものとして計算しているので、この警告が出る開傘は、パラシュートが壊れて弾道落下に近い降下になりうるものとして扱う。

6. **空力の手法の違いを含める。**
   空力係数は実測と比較しておらず（:ref:`sec-lim-aero`）、手法の差は最大着地距離に大きく効く。
   上のコマンドのように、``aero.method = "barrowman"`` と ``"panel"`` の両方で手順 1〜5 を計算し、大きいほうの最大値を使う。
   公称の飛翔の着地点の差（サンプルの弾道落下で約 66 m）は小さくても、風の格子とモンテカルロの最大値の差は大きくなりうる。
   ``cn_scale`` の標準偏差は一つの手法の中の不確かさで、手法の差はこの手順で含める。

7. **終了ステータスと警告を確かめ、半径を決める。**
   ``runs/status.txt`` が空であること（すべてのコマンドの終了ステータスが 0 であること）を確かめる。ここに書かれた計算の結果は保安の判断に使わない（:ref:`sec-exit-status`）。
   着地しなかったケース、失敗したケース、静的に不安定なケースや安定を判定できないケース、係数表の迎角の範囲外の引き出しがあると、終了ステータス 3 になり、標準エラー出力に ``INCOMPLETE RESULTS`` または ``RESULTS NOT USABLE`` の行が出る。
   失敗や着地しないケースがあるときの最大着地距離は下限である（:ref:`sec-wind-grid-failed`）。
   警告は ``-q`` でも表示し、``dispersion_meta.json`` と ``dispersion_summary.json`` の ``warnings`` に残る（:ref:`sec-config-checks`）。開傘の速さ、静安定余裕の不足、``power`` と ``log`` の風の上空への外挿、表の拡大率、表の最上端より高く上がったケースも見る。
   各計算の最大着地距離は、風の格子では ``dispersion.csv``、モンテカルロ解析では ``dispersion_summary.json`` の ``max_landing_distance`` にある。次のようにまとめて読める。

   .. code-block:: bash

      cat runs/status.txt   # 何も出なければよい
      python3 - <<'EOF'
      import csv, glob, json
      for f in sorted(glob.glob("runs/*/dispersion.csv")):
          rows = [r for r in csv.DictReader(open(f))
                  if r["landed"] == "true"]
          for d in ("ballistic", "parachute"):
              v = [float(r["landing_distance"])
                   for r in rows if r["descent"] == d]
              if v:
                  print(f, d, max(v))
      for f in sorted(glob.glob("runs/*/dispersion_summary.json")):
          for d in json.load(open(f))["descents"]:
              print(f, d["descent"], d["max_landing_distance"],
                    d["landing_distance_p99"], d["complete"])
      EOF

   半径は、すべての最大値の最大に余裕を足した値とする。

   .. math::

      R = \max\bigl(\text{手順 1〜6 のすべての計算の、弾道落下とパラシュート降下の最大着地距離}\bigr) + \text{余裕}

   余裕には、風は定常で突風や乱流を含まないこと、地形を考えず着地を射点標高の面で判定すること（:doc:`limitations`）、空力係数が未検証であることを見込む。余裕の大きさは利用者が決める。

サンプル数と最大値
~~~~~~~~~~~~~~~~~~

モンテカルロ解析の最大着地距離は、サンプル数とともに大きくなる。
ばらつきは正規分布で、その裾には上限がないので、サンプルを増やすほど極端な値（たとえば標準偏差の 4〜5 倍の軸力の減少）を引くからである。
相対的なばらつき（``thrust_scale``、``cn_scale``、``ca_scale`` などの倍率）も、裾を切らない正規分布から引く。倍率には 0.05 の下限（:eq:`eq-mc-factor`）があるだけなので、
``ca_scale`` の :math:`-0.72`\ （標準偏差 0.15 の 4.8 倍、軸力が 0.28 倍）のような、現実には起こりにくい値も引かれ、それが最大値を決めることがある。
最大値を決めているサンプルのばらつきは、下の 3 つ目の項目のように確かめる。
サンプル :math:`i` の値は ``seed + i`` だけで決まるので（:ref:`sec-mc`）、サンプル数を増やした計算は少ない計算のサンプルをすべて含み、最大値は減ることがない。
一方、P99 などのパーセンタイルはサンプル数とともに一定の値に収まる。
:numref:`tbl-safety-samples` に、サンプル（``power`` の風）の手順 3 の 2 つのケースで、サンプル数を変えた結果を示す。P99.9 は ``dispersion_mc.csv`` の ``landing_distance`` から :eq:`eq-mc-percentile` で求めた値である。

.. _tbl-safety-samples:

.. list-table:: サンプル数と着地距離 [m]（サンプル、``power`` の風、手順 3 の機体のばらつきだけ、``seed = 1``。弾道落下はパネル法で 4.25 m/s、90°、パラシュート降下（表の「パラシュート」）は部品積み上げ法で 7 m/s、90°）
   :header-rows: 1
   :widths: 13 12 12 12 17 17 17

   * - サンプル数
     - 弾道落下 P99
     - 弾道落下 P99.9
     - 弾道落下 最大
     - パラシュート P99
     - パラシュート P99.9
     - パラシュート 最大
   * - :math:`10^3`
     - 3915.7
     - 4552.5
     - 4826.3
     - 20265.2
     - 24164.2
     - 28928.2
   * - :math:`10^4`
     - 3906.1
     - 4503.8
     - 5000.6
     - 20151.6
     - 23085.7
     - 28928.2
   * - :math:`4\times10^4`
     - 3897.9
     - 4421.0
     - 5221.1
     - 20105.9
     - 23236.4
     - 28928.2
   * - :math:`10^5`
     - 3910.1
     - 4454.5
     - 7673.6
     - 20183.5
     - 23271.9
     - 36276.8

P99 と P99.9 は :math:`10^3` から :math:`10^5` サンプルで数 % しか変わらないが、最大値は :math:`10^5` サンプルで大きく増える。
:math:`10^5` サンプルのパラシュート降下の最大（61243 番）は、軸力の倍率が 0.28（``ca_scale`` の :math:`-0.72`、標準偏差 0.15 の 4.8 倍）で頂点が 10937 m まで上がった飛翔である。
したがって、次のようにする。

* サンプル数は :math:`10^4` 以上にし、P99 と P99.9 と最大値をあわせて報告する。P99.9 は :math:`10^4` サンプルでは上位 10 個ほどの点で決まる。
* サンプル数を 2〜4 倍にした計算と比べて、P99 と P99.9 が変わらないことを確かめる。最大値は収まらないので、サンプル数とともに記録する。
* 最大値を決めている飛翔（``max_landing_distance_sample`` の行）のばらつきを ``dispersion_mc.csv`` で見て、その組合せが起こりうるものかを判断する。正規分布の遠い裾が現実的でない量は、標準偏差の根拠とあわせて見直す。

最悪の重ね合わせ
~~~~~~~~~~~~~~~~

手順 3 と 5 は、上限の風、機体のばらつきの裾、回収系の故障という、それぞれ起こりにくい条件を重ねる。
同時に起こる確率は小さく、得られる半径は意図して保守的なものである。どの条件までを重ねるか（たとえば上限の風とメインが頂点で開く故障を同時に考えるか）は、打上げを行う団体が決める。
重ねた結果の半径は、頂点高度の数倍になりうる（下の計算例では、頂点高度約 4.4 km に対してパラシュート降下の最大が 28.9 km）。

設定のそろえ方
~~~~~~~~~~~~~~

``sim``、風の格子、モンテカルロ解析は、``[sim]`` を含む設定（``[dispersion]``、``[output]``、``[resources]`` を除く）を同じにする。
KML は、設定のハッシュが現在の設定と一致する結果だけをまとめるので（:ref:`sec-kml`）、積分法などが違うと、飛翔経路・風の格子・モンテカルロの層が 1 つの KML にそろわない。
上の手順は計算ごとに ``--out`` で出力先を分けるので、KML も計算ごとに別になる。``--set`` で変えた値はハッシュに入るので、同じ出力先で ``sim`` と ``dispersion`` をそろえるときは、同じ ``--set`` を与える。
どの計算も、積分法などの ``[sim]`` の設定はそろえる。

計算例
~~~~~~

サンプル（射角 85°、射方位 270°）について、手順 1〜6 を計算した結果を :numref:`tbl-safety-example` に示す。
この表は、再現できるように、高度別の表ではなくサンプル設定の ``power`` の風（風速は地上 2 m、上限 7 m/s）で計算した。実際の手順では、当日の予報の高度別の表を使う。
コマンドは上の手順で ``P=(--set sim.integrator=rk45)``、``N=()``、``G=(--set 'dispersion.wind_speeds=[1,2,3,4,5,6,7]')`` としたもので、
手順 3 の風は ``wind.profile_mean_speed`` と ``wind.profile_mean_direction_deg`` の代わりに ``--set wind.speed=7 --set wind.direction_deg=270`` のように ``[wind]`` の ``speed`` と ``direction_deg`` で与える（``power`` ではこの 2 つが基準風である）。
手順 3 の最悪の風での計算は、表ではパネル法の弾道落下の最悪の風だけを示す。
モンテカルロ解析は :math:`10^4` サンプル、``seed = 1``、標準偏差はファイルの値である（手順 3 では風の標準偏差を 0 にした）。

.. _tbl-safety-example:

.. list-table:: 保安距離の計算例（サンプル、``power`` の風、最大着地距離 [m]。括弧内はその風速と風向）
   :header-rows: 1
   :widths: 46 27 27
   :class: longtable

   * - 計算
     - 弾道落下
     - パラシュート降下
   * - 手順 1：風の格子（1〜7 m/s、36 方位）、部品積み上げ法
     - 1550.7（7 m/s、270°）
     - 14314.1（7 m/s、90°）
   * - 手順 1：同、パネル法
     - 2861.2（4 m/s、90°）
     - 13496.6（7 m/s、90°）
   * - 手順 2：細分した格子（3〜5 m/s を 0.25 m/s 刻み、5〜7 m/s を 0.5 m/s 刻み、360 方位）、パネル法
     - 2943.2（4.25 m/s、90°）
     - 13496.6（7 m/s、90°）
   * - 手順 3：7 m/s、30° ごとの 12 風向、機体のばらつきだけ、部品積み上げ法
     - 3994.9（270°）
     - 28928.2（90°）
   * - 手順 3：同、パネル法
     - 3930.0（270°）
     - 26244.8（90°）
   * - 手順 3：パネル法の弾道落下の最悪の風（4.25 m/s、90°）、パネル法
     - 5000.6
     - 16354.8
   * - 手順 4：予報の風（北風 4 m/s、風のばらつきを含む）、部品積み上げ法
     - 2581.5
     - 19119.0
   * - 手順 4：同、パネル法
     - 2543.4
     - 17741.8
   * - 最大
     - 5000.6
     - 28928.2

部品積み上げ法の格子の最大値は、36、72、360 方位でも、3〜7 m/s を 0.25〜0.5 m/s 刻みにしても変わらない（1550.7 m と 14314.1 m）。
パネル法の弾道落下の最大は上限の 7 m/s ではなく 4 m/s 付近で起き、細分すると 2861.2 m から 2943.2 m に増える。
手順 3 のパラシュート降下の最大 28928.2 m は、格子の最大 14314.1 m の約 2 倍である。最も遠いサンプル（830 番）は軸力の倍率が 0.56（``ca_scale`` の :math:`-0.44`）で頂点が 7326 m まで上がり、上限の風で長く流される。
この着地点は、公称の機体を飛ばす風の格子にも、北風 4 m/s の周りのモンテカルロ解析にも現れない。
パネル法の弾道落下の最悪の風（4.25 m/s、90°）で機体のばらつきを与えると、弾道落下の最大は 5000.6 m になり、格子の値の 1.7 倍である。
手順 5 はサンプルでは弾道落下だけである（単一のパラシュート）。

したがって、半径は :math:`\max(5000.6,\ 28928.2)=28928.2` m に余裕を足した値になる。
余裕を 10 % とみなせば :math:`28928.2\times1.1\approx31821` m である（10 % は計算例であり、推奨値ではない）。
手順 3 を省くと（予報の風の周りのモンテカルロ解析と、7 m/s までの格子だけでは）最大は 19119.0 m で、手順 3 を含めた値の約 2/3 にとどまる。
この最大値は :math:`10^4` サンプルでの値で、サンプル数を増やすと大きくなる（:numref:`tbl-safety-samples`）。

高度別の表の例として、上の手順のコマンド（``examples/wind_profile_example.csv``、地上 2 m で 7 m/s を上限とする格子 7.8〜27.4 m/s）で手順 1〜3 を計算した結果を :numref:`tbl-safety-profile` に示す（``P`` に ``--set sim.integrator=rk45`` を加え、``N=()`` とした）。

.. _tbl-safety-profile:

.. list-table:: 高度別の表での手順 1〜3 の計算例（``examples/wind_profile_example.csv``、最大着地距離 [m]。括弧内は平均風速と平均風の風向。手順 3 は :math:`10^4` サンプル、機体のばらつきだけ）
   :header-rows: 1
   :widths: 46 27 27
   :class: longtable

   * - 計算
     - 弾道落下
     - パラシュート降下
   * - 手順 1：風の格子（7.8〜27.4 m/s、36 方位）、部品積み上げ法
     - 2890.8（27.4 m/s、110°）
     - 10566.0（27.4 m/s、130°）
   * - 手順 1：同、パネル法
     - 2832.8（27.4 m/s、120°）
     - 10001.4（27.4 m/s、140°）
   * - 手順 2：360 方位、部品積み上げ法
     - 2891.5（27.4 m/s、113°）
     - 10566.1（27.4 m/s、131°）
   * - 手順 2：同、パネル法
     - 2834.4（27.4 m/s、4°）
     - 10001.4（27.4 m/s、139°）
   * - 手順 3：部品積み上げ法の弾道落下の最悪の風（27.4 m/s、113°）
     - 5227.2
     - 23963.2
   * - 手順 3：部品積み上げ法のパラシュート降下の最悪の風（27.4 m/s、131°）
     - 5056.8
     - 23738.3
   * - 手順 3：パネル法の弾道落下の最悪の風（27.4 m/s、4°）
     - 3713.5
     - 20892.3
   * - 手順 3：パネル法のパラシュート降下の最悪の風（27.4 m/s、139°）
     - 4794.6
     - 20549.4
   * - 手順 3：27.4 m/s、30° ごとの 12 風向、部品積み上げ法
     - 5271.8（90°）
     - 24406.6（30°）
   * - 手順 3：同、パネル法
     - 5043.3（90°）
     - 21117.4（60°）
   * - 最大
     - 5271.8
     - 24406.6

格子では、どちらの降下モードも上限の 27.4 m/s で最も遠く、平均風の風向が南東（110〜140°）のときに最大になる。パネル法の弾道落下は、360 方位にすると 120° 付近と 4° 付近の二つの山がほぼ同じ高さになり、最大の風向が 4° に移る。
この風で機体のばらつきを与えると、弾道落下の最大は 3713.5 m だが、パラシュート降下の最悪の風（139°）では弾道落下が 4794.6 m になり、30° ごとの風向では 90° で 5043.3 m になる。
格子で最も遠い風向が、機体のばらつきを与えたときにも最も遠いとは限らないので、ほかの風向も計算する。
パラシュート降下の最大は、部品積み上げ法の 30° の 24406.6 m で、格子の最大 10566.1 m の約 2.3 倍である（最も遠いのは ``power`` の例と同じ 830 番のサンプル）。
これらの計算では、モンテカルロ解析の ``wind_scale`` が 2.31（:math:`27.4/11.89`）で、公称の風が格子の 27.4 m/s のケースと同じであることを ``dispersion_summary.json`` で確かめた。

二段開傘の例として、サンプルの回収系をドローグ 0.5 m\ :sup:`2`、高度 300 m でメイン 8 m\ :sup:`2` に変えた機体（上の「準備」の ``P+=`` と ``N=``）について、
正常な開傘（手順 2 と 3）と 2 つの故障（手順 5）のパラシュート降下の最大着地距離を :numref:`tbl-safety-dual` に示す。
上の ``power`` の例と同じく、``P=(--set sim.integrator=rk45 --set recovery.cd_s=0.5)``、格子を ``power`` の 1〜7 m/s、空力を部品積み上げ法とし、
モンテカルロ解析の風は ``wind.speed`` と ``wind.direction_deg`` で与えた（:math:`10^4` サンプル、``seed = 1``、風の標準偏差は 0）。

.. _tbl-safety-dual:

.. list-table:: 二段開傘の計算例（``power`` の風、部品積み上げ法、パラシュート降下の最大着地距離 [m]。格子は 1〜7 m/s、360 方位で、どの開傘でも 7 m/s、風向 90° で最も遠い。MC はその風でのモンテカルロ解析。着地は北風 4 m/s の ``sim`` の着地時刻 [s]。メインが頂点で開く場合は ``main_altitude = 100000``）
   :header-rows: 1
   :widths: 40 15 15 15 15

   * - 開傘
     - 格子
     - MC 最大
     - MC P99
     - 着地
   * - 正常（300 m でメイン）
     - 6187.7
     - 11974.7
     - 8503.4
     - 331.9
   * - メインが頂点で開く
     - 21624.4
     - 43650.0
     - 30439.9
     - 1021.5
   * - メインが開かない（ドローグだけ）
     - 5429.2
     - 11039.8
     - 7654.9
     - 276.5

格子の最大値は、36 方位でも 360 方位でも同じである。弾道落下は回収系の設定によらず、どの開傘でも格子の最大は 1550.7 m（7 m/s、270°）である。
ドローグだけの降下は正常な開傘より約 55 s 早く着地し、最大着地距離も短い（着地速度は 17.0 m/s で、15 m/s を超える警告が出る）。
メインが頂点で開くと降下時間は約 3 倍になり、格子の最大は 21624.4 m、その風でのモンテカルロ解析の最大は 43650.0 m で、この機体ではこれが半径を決める。
どのモンテカルロ解析でも、最大を決めているのは 830 番のサンプル（:numref:`tbl-safety-example` の手順 3 と同じ、頂点が高く上がるサンプル）である。
どの場合もドローグは頂点の 1 s 後に 28.4 m/s、動圧 0.32 kPa で開き、正常な開傘のメインは 16.3 m/s、0.16 kPa で開く（``sim`` の ``deploy`` と ``main deploy`` の行）。開傘の速さの警告は出ない。
