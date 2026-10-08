.. _page-theory-dispersion:

落下分散
==========

落下分散は、条件の違いによって着地点がどれだけ広がるかを示す。保安距離の設定など、打上げの安全を判断するための基本的な資料になる。
この章は、風だけを変える風の格子と、機体・推力・設置のばらつきを同時に与えるモンテカルロ解析の違い、サンプル数と統計量の読み方、標準偏差の与え方を述べる。
計算の手順は :ref:`sec-dispersion-safety`、実行と結果の読み方は :ref:`page-howto-dispersion-run` にある。

この章で決めること
------------------

* 解析の方式：:ref:`dispersion.mode <cfg-dispersion-mode>`\ （``"wind_grid"`` または ``"monte_carlo"``）
* 風の格子の細かさ：:ref:`dispersion.wind_speeds <cfg-dispersion-wind_speeds>`、:ref:`dispersion.directions <cfg-dispersion-directions>`
* サンプル数と乱数の種：:ref:`dispersion.monte_carlo.samples <cfg-dispersion-monte_carlo-samples>`、:ref:`dispersion.monte_carlo.seed <cfg-dispersion-monte_carlo-seed>`
* 乱数の切り詰め：:ref:`dispersion.monte_carlo.truncate_sigma <cfg-dispersion-monte_carlo-truncate_sigma>`
* ばらつきの標準偏差（20 項目）：:ref:`dispersion.monte_carlo.* <cfg-dispersion-monte_carlo-thrust_scale>`\ （:numref:`tbl-mc-perturb`）

選び方
------

:numref:`fig-dispersion-choice` は、知りたいことから方式を選ぶ流れである。

.. _fig-dispersion-choice:

.. figure:: /_generated/tikz/dispersion_choice.*
   :width: 90%

   落下分散の方式の選び方。見る点は、風の影響を見通すだけなら ``wind_grid`` で足り、機体のばらつきを含む分布が要るときは ``monte_carlo`` が要ること、
   保安距離の半径は両方を手順に沿って組み合わせて決めることである。

.. _tbl-choose-dispersion:

.. list-table:: 方式の比較
   :header-rows: 1
   :widths: 12 22 24 22 20

   * - 方式
     - 変えるもの
     - 得られる量
     - 仮定と限界
     - コスト（サンプル機体）
   * - ``wind_grid``
     - 基準風速 :math:`K` 通り × 風向 :math:`J` 通り × 降下モード 2 つ。機体は公称のまま
     - 最大着地距離、風向ごとの着地点の多角形。最悪の風向と風速
     - 風は定常。機体のばらつきは入らない。最大値は計算した格子の中の最大で、多角形は隣り合う風向を直線で結んだ近似
     - 112 ケース（7 風速 × 8 方位 × 2）で約 0.7 s（``rk4``）
   * - ``monte_carlo``
     - 20 項目のばらつきを同時に引く。:math:`N` サンプル × 降下モード 2 つ
     - 平均、誤差楕円、P50・P90・P99・P99.9、最大着地距離
     - ばらつきは互いに独立な正規分布（相関なし）。標準偏差は利用者が決める。最大値はサンプル数とともに増える
     - 1000 サンプルで約 0.8 s（``rk45``）、:math:`10^4` サンプルで約 13 s

コストは、20 スレッドの CPU（Intel Core i7-12700、ほかの処理も動いていた）での値である。
モンテカルロ解析 1 回（:math:`10^4` サンプル、``rk45``）は 4 スレッドでは約 32 s、``sim.integrator = "rk4"``\ （``dt`` = 2 ms）では 20 スレッドで約 170 s かかる。
保安距離の手順（風の格子 7 回と :math:`10^4` サンプルのモンテカルロ解析 30 回）は、``rk45`` で約 8 分である。

風の格子の細かさ
~~~~~~~~~~~~~~~~

多角形は隣り合う風向の着地点を直線で結ぶので、風向の間隔が粗いと、線の外側にも着地点が出る。
サンプルの風速 7 m/s で風向を 8 から 72 に増やすと、8 方位の多角形の外側に出る着地点の距離は最大で、パラシュート降下が約 1050 m、弾道落下が約 48 m である。
一方、サンプルの最大着地距離は 8、36、72 方位で同じである（パラシュート降下 14315.6 m、弾道落下 1549.6 m。最大が東西の風向にあり、どの方位数にも含まれるため）。
ただし最大着地距離は計算した風速と風向の中の最大値なので、一般には方位数と風速の刻みで変わる。
判断の目安は次のとおりである。

* 保安域の判断には、多角形ではなく最大着地距離を半径とする円か、方位を 36 以上にした外側の包絡を使う。
* 格子を細かくして（方位を増やし、風速の刻みを小さくして）最大値が変わらないことを確かめる。
* 着地しなかったケースは最大着地距離から除かれ、最大値は下限になる（:ref:`sec-wind-grid-failed`）。

サンプル数
~~~~~~~~~~

:numref:`tbl-choose-samples` は、着地したサンプル数 :math:`n` と使える統計量の関係である。
サンプル数の根拠になる数値は :numref:`tbl-safety-samples` にある。

.. _tbl-choose-samples:

.. list-table:: サンプル数と統計量
   :header-rows: 1
   :widths: 18 82

   * - 着地したサンプル数
     - 使える統計量と注意
   * - 1
     - 平均の着地点だけが求まる。共分散、誤差楕円、頂点高度の標準偏差は求まらない。
   * - 2 以上 30 未満
     - 共分散と誤差楕円が求まるが、統計は粗い（警告が出る）。
   * - 30 以上 1000 未満
     - 平均、誤差楕円、P50・P90・P99 が使える。P99.9 は求めない（``n/a``）。
   * - 1000
     - P99.9 を出す最小の数。上位 2 個ほどの点で決まる粗い値である。
   * - :math:`10^4` 以上
     - P99 と P99.9 を報告する。P99.9 は上位 10 個ほどの点で決まる。サンプル数を 2〜4 倍にした計算と比べて、P99 と P99.9 が変わらないことを確かめる。

最大着地距離は、サンプルを増やすほど大きくなり、収束しない。
:math:`10^3` から :math:`10^5` サンプルで P99 と P99.9 は数 % しか変わらないが、最大値は :math:`10^5` サンプルで大きく増える（:numref:`tbl-safety-samples`）。
したがって、P99 と P99.9 と最大値を、サンプル数とあわせて報告する。

.. _fig-mc-sample-convergence:

.. figure:: /_generated/plots/mc_sample_convergence.*
   :width: 80%

   サンプル数 :math:`N`\ （横軸）に対する着地距離の最大、P99.9、P99（左は弾道落下、右はパラシュート降下。条件は :numref:`tbl-safety-samples`）。
   見る点は、P99 と P99.9 が一定の値に収まるのに対し、最大値は増え続けることである。

乱数の切り詰め
~~~~~~~~~~~~~~

既定では、標準正規乱数を切らずに引く。裾に上限がないので、サンプルを増やすほど極端な値を引き、それが最大値を決める。
``truncate_sigma`` を指定すると、乱数を :math:`[-k,k]` に切る（:numref:`fig-mc-perturbation`）。
:numref:`tbl-choose-truncate` は、切った場合と切らない場合の違いである。

.. _tbl-choose-truncate:

.. list-table:: ``truncate_sigma`` の影響（サンプル、パラシュート降下、:math:`10^5` サンプル、着地距離 [m]）
   :header-rows: 1
   :widths: 30 23 23 24

   * - 設定
     - P99
     - P99.9
     - 最大
   * - 切らない
     - 20183.0
     - 23271.7
     - 36278.6
   * - ``truncate_sigma`` = 3
     - 20151.6
     - 23009.8
     - 28927.8

条件は :numref:`tbl-safety-samples` と同じである。切らないときの最大は、軸力の倍率が 0.28（``ca_scale`` の :math:`-0.72`、標準偏差 0.15 の 4.8 倍）になった 1 サンプルで決まる。
3 倍で切ると、このサンプルの値は 3 倍に切られ、最大は :math:`10^4` サンプルの計算と同じ値になる。
判断の目安は次のとおりである。

* P99 と P99.9 は切っても変わらない。変わるのは、遠い裾で決まる最大値だけである。
* 現実には起こりにくい値（標準偏差の 4〜5 倍）が最大を決めているときに使う。標準偏差の根拠とあわせて、切る範囲も仮定として報告する。
* 切らない計算と切った計算の両方を残し、差を確かめる。

標準偏差の与え方
~~~~~~~~~~~~~~~~

既定値は目安にすぎず、標準偏差は利用者が決める（:ref:`sec-lim-uncertainty`）。:numref:`tbl-choose-sigma` に、群ごとの既定値と注意をまとめる。各項目の反映のしかたは :numref:`tbl-mc-perturb` にある。

.. _tbl-choose-sigma:

.. list-table:: ばらつきの群と標準偏差
   :header-rows: 1
   :widths: 12 30 22 36

   * - 群
     - 項目（既定値）
     - 既定で有効か
     - 決め方の注意
   * - 推力
     - ``thrust_scale`` (0.03)、``burn_time_scale`` (0.02)、``thrust_misalignment_deg`` (0)、``thrust_offset`` (0)
     - 上の 2 つ
     - 傾きとオフセットは、非対称を扱うときだけ与える。
   * - 質量・重心
     - ``dry_mass`` (0.1)、``cg`` (0.01 m)、``cg_lateral`` (0)
     - 上の 2 つ
     - 横ずれを与えると機体が軸対称でなくなり、ロール共振の確認が働く。
   * - 空力
     - ``cn_scale`` (0.10)、``ca_scale`` (0.15)、``cl_scale``、``clp_scale``、``lag_scale``、``magnus_scale``、``cmq_scale``、``fin_cant_deg`` (すべて 0)
     - 上の 2 つ
     - ``cn_scale`` は 1 つの推算法の不確かさで、手法どうしの差は覆わない。サンプルではフィンの法線力が Barrowman でパネル法・CFD より 23〜32 % 低いので、両方の手法で計算して大きいほうを使う（:ref:`sec-dispersion-safety` の手順 6）。
       危険区域の解析では ``cmq_scale`` を 0.1、``lag_scale`` を 0.25 にする（:ref:`sec-quasi-steady`）。``magnus_scale`` は表に値があるときだけ与える。
   * - 風・設置
     - ``wind_speed`` (1.0 m/s)、``wind_direction_deg`` (15°)、``elevation_deg`` (0.5°)、``azimuth_deg`` (1.0°)
     - すべて
     - 風のばらつきは基準風の周りである。最悪の風を格子で決めてからモンテカルロ解析を行うときは、その風の周りに与える。
   * - 回収
     - ``parachute_cd_s_scale`` (0.1)
     - 有効
     - ``cd_s`` と ``main_cd_s`` の両方に同じ倍率を掛ける。

保安距離に使う統計量
~~~~~~~~~~~~~~~~~~~~

:numref:`tbl-choose-statistic` は、統計量ごとの前提とサンプル数への依存である。

.. _tbl-choose-statistic:

.. list-table:: 統計量の使い方
   :header-rows: 1
   :widths: 16 26 26 32

   * - 統計量
     - 前提
     - サンプル数への依存
     - 使い方
   * - 最大着地距離
     - なし
     - 増え続ける。失敗したサンプルがあると下限
     - 記録して報告する。計算した条件の中の最大で、計算していない条件はこれより遠くなりうる
   * - P99、P99.9
     - なし（分布の形によらない）
     - :math:`10^4` 以上で数 % の範囲に収まる
     - 半径を考える目安にする。P99.9 は 1000 以上で求まる
   * - 誤差楕円
     - 着地点が二次元の正規分布に従う
     - 少ないと不安定
     - 分布の広がりの要約。1σ 楕円の内側は約 39 %、3σ は約 99 %。歪むときは使わない
   * - 平均
     - なし
     - 少なくてよい
     - 分布の中心の位置

半径は、すべての計算の最大着地距離（弾道落下とパラシュート降下）の最大に、余裕を足した値とする。

.. math::

   R = \max\bigl(\text{すべての計算の、弾道落下とパラシュート降下の最大着地距離}\bigr) + \text{余裕}

余裕には、風は定常で突風や乱流を含まないこと、地形を考えず着地を射点標高の面で判定すること（:ref:`page-theory-limitations`）、空力係数が未検証であることを見込む。
余裕の大きさは利用者が決める。保安距離そのものの規定は、打上げを行う場所と団体の規則に従う。
手順は :ref:`sec-dispersion-safety` にある。

モデル
------

風の格子（wind_grid）
~~~~~~~~~~~~~~~~~~~~~


基準風速の一覧 :math:`\{w_1,\dots,w_K\}`\ （既定 1〜7 m/s）と、北から等間隔の :math:`J` 方位（既定 8 方位）

.. math::

   \psi_{w,j} = \frac{360^\circ\,j}{J},\qquad j=0,1,\dots,J-1

のすべての組合せについて、飛翔計算を行い着地点を求める。
パラシュートが有効な場合は、正常に開傘した場合（パラシュート降下）と、開傘しなかった場合（弾道落下）の両方を計算する。
したがってケース数は :math:`K\times J\times2` である。

ケースごとの風は、``[wind]`` で選んだ風モデルの基準風速と基準風向だけを差し替えて作る。
``constant``、``power``、``log`` では、格子の風速は ``[wind]`` の基準高さ ``ref_height`` での風速である。べき法則と対数則では、この風速を上空へ外挿するので、頂点が地上 1000 m を超えると警告を出す（:ref:`IY-O514`）。
高度別の表（``profile``）では、格子の風速は **表の平均風速** :math:`W`\ （最下行から最上行までの高度範囲での平均）で、格子の風向は表の平均風の風向 :math:`\Psi` である。
表は、形（高度ごとの風速比と風向のねじれ）を保ったまま、:math:`k=v/W` 倍に拡大し、:math:`\psi-\Psi` だけ回転する（:ref:`sec-wind-scaling`）。
地上 2 m の風速ではないので、地上風の測定値を ``wind_speeds`` にそのまま入れてはならない。
:math:`W` は表の全範囲での平均なので、表を飛翔の高度範囲に切ると、``wind_speeds`` や標準偏差の解釈が容易になる。
切るときは、公称の頂点のすぐ上ではなく、格子とモンテカルロ解析で予想される最も高い頂点（たとえば公称の頂点 + 3σ）より上まで残す（:ref:`sec-wind-profile`）。:math:`W` は ``ignisyeet sim`` の ``summary.json`` の ``wind_speed`` に出る。
平均風速がほぼ 0 の表はこの拡大ができないので、エラーにする。

.. _sec-mc:

モンテカルロ解析（monte_carlo）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

実際の落下点は、風だけでなく推力のばらつき、機体の質量、空力係数の推算誤差、ランチャの設置誤差など多くの要因でばらつく。
モンテカルロ解析は、これらを確率変数として多数回の飛翔計算を行い、着地点の分布を統計量で表す :cite:`metropolis`。

サンプル数を :math:`N`\ （``samples``）として、サンプル :math:`i=0,\dots,N-1` ごとに 20 項目のばらつきを引き（:numref:`tbl-mc-perturb`）、
弾道落下と（パラシュートが有効なら）パラシュート降下の両方を **同じ乱数で** 飛ばして、着地点などを記録する。
サンプルは互いに独立で、結果は計算の順序やスレッド数によらない。:numref:`fig-mc-perturbation` に流れを示す。

.. _fig-mc-perturbation:

.. figure:: /_generated/tikz/mc_perturbation.*
   :width: 100%

   モンテカルロ解析のばらつき。乱数に標準偏差を掛けたものが 20 項目（五つの群）のばらつきになり、飛翔に反映される。
   下の図は ``truncate_sigma`` = :math:`k` で乱数を切り詰めたときの分布で、切った外側の確率が両端の点に集まる。



乱数
~~~~

各サンプルの乱数は ``seed`` とサンプル番号 :math:`i` だけで決まる。したがって、次のことが成り立つ。

* 同じ ``seed`` と設定なら、毎回同じ結果になる。
* ある項目の標準偏差を変えても（0 にしても）、ほかの項目の値は変わらない。1 項目だけの影響を調べるときに、サンプルどうしを比較できる。
* サンプル数を増やすと、前の計算のサンプルをすべて含む。
* ``seed`` が 1 違う計算は、サンプル番号が 1 ずれた同じ値を引く。独立な計算にするには、``seed`` を :math:`N` 以上離す。

ばらつきは、標準正規乱数 :math:`z_k\sim N(0,1)` に標準偏差 :math:`\sigma_k` を掛けた :math:`\epsilon_k=\sigma_k z_k` である。

標準正規乱数 :math:`z` は、既定では切らない（裾に上限のない）正規分布から引く。``truncate_sigma``\ （:math:`k`。省略時はなし）を指定すると、各 :math:`z` を :math:`[-k,k]` に切り詰め（:math:`z\leftarrow\max(-k,\min(k,z))`）、それから標準偏差を掛ける。
相対のばらつきにも絶対のばらつきにも同じく効き、どのばらつきも標準偏差の :math:`k` 倍を超えない。
引き直すのではなく端の値に切り詰めるので、引く数と順序は変わらず、:math:`[-k,k]` の内側の値と、ほかのサンプルの値はそのまま保たれる。:math:`k` の外側の値はちょうど :math:`\pm k` になるので、分布は端に点の重みを持つ。

:math:`k` は有限で正の数でなければならず、そうでなければ設定の読み込みでエラーになる（IY-C032）。

使った値は ``dispersion_summary.json`` の ``truncate_sigma`` に残る（切らないときは ``null``）。

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
   * - ``lag_scale``
     - 相対
     - 0
     - 遅れの和 :math:`L_0,L_1`\ （胴体とフィンの見かけの質量の遅れと対流の回転の項。:eq:`eq-lag-moment`、:eq:`eq-lag-conv-force`）に :math:`f(\epsilon)` を掛ける
   * - ``magnus_scale``
     - 相対
     - 0
     - Magnus 力とモーメントの傾き ``cnpa``、``cmpa``\ （:eq:`eq-magnus`、:eq:`eq-magnus-moment`）に :math:`f(\epsilon)` を掛ける。表にこれらの値がなければ効かず、警告を出す
   * - ``cmq_scale``
     - 相対
     - 0
     - ピッチとヨーの減衰の和（一般表では :math:`S^q` と :math:`S^r`）に、``cn_scale`` の倍率に重ねて :math:`f(\epsilon)` を掛ける。減衰のモーメントと回転による法線力（:eq:`eq-cnq-force`）が変わる

燃焼時間と推力
~~~~~~~~~~~~~~

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
~~~~~~~~

空力のばらつきは、係数表の **引いたあとの値** に掛ける。\ :eq:`eq-aero-force` の :math:`C_N` に加え、
:math:`C_{N\alpha}` とピッチ減衰の和 :math:`S_0,S_1,S_2`\ （:eq:`eq-moment-damp`）も同じ倍率 :math:`f_{N}` で変えるので、
圧力中心 :math:`x_{cp}` は変わらず、復元モーメントとピッチ減衰がともに :math:`f_N` 倍になる。
軸力は :math:`C_A^{\mathrm{on}}` と :math:`C_A^{\mathrm{off}}` の両方を :math:`f_A` 倍する。
``cl_scale`` は :math:`C_l` と :math:`C_{l\delta}` と誘導ロールの :math:`C_{l4}` を、``clp_scale`` は :math:`C_{lp}` を同様に倍率で変える。
``cmq_scale`` は減衰の和だけに :math:`f_N` に重ねて掛け、:math:`C_N` と圧力中心は変えない。``lag_scale`` は遅れの和、``magnus_scale`` は ``cnpa`` と ``cmpa`` に掛ける（これらの項の式は :ref:`sec-aero-unsteady`）。
これらの倍率は ``sim.quasi_steady`` の設定に重ねて掛けるので、準定常の設定では ``lag_scale`` と ``magnus_scale`` は効かない。
危険区域を求める解析では ``cmq_scale`` を 0.1、``lag_scale`` を 0.25 にすることを勧める（:ref:`sec-quasi-steady`）。
``cmq_scale`` は局所的な迎角の和（減衰のモーメントと回転による法線力）だけに掛かり、サンプルのピッチ減衰の 16〜29 % を占める見かけの質量の項（遅れの和）には掛からない。
後者の不確かさ（フィンの見かけの質量を Barrowman の傾きにそろえる仮定。細長い物体の理論のままなら 1.47 倍）は ``lag_scale`` で見込む。
循環の遅れとモデルの差は向きの決まった偏りで、平均 0 のばらつきでは表せない（:ref:`sec-quasi-steady`）。
ロールの係数の倍率は :math:`C_l^{\mathrm{tot}}`\ （:eq:`eq-cl-total`）の強制項と減衰項に別々に効くので、
定常なロール角速度（:eq:`eq-roll-steady`）は :math:`f_{Cl}/f_{Clp}` 倍になる。

.. _sec-mc-rail:

レールの向き
~~~~~~~~~~~~

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
~~~~~~~~~~~~~~~~~~~~~~~~

``thrust_misalignment_deg``、``thrust_offset``、``fin_cant_deg``、``cg_lateral``、``cl_scale``、``clp_scale`` の標準偏差の既定値は 0 で、
指定したときだけ有効になる（0 のときは公称値のままである）。空力の ``lag_scale``、``magnus_scale``、``cmq_scale`` も同じく既定値は 0 である。

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

.. figure:: /_generated/tikz/error_ellipse.*
   :width: 75%

   着地点の分布（点）と誤差楕円。実線が :math:`1\sigma`、破線が :math:`3\sigma` で、
   長軸の方位 :math:`\beta` は北から時計回りに測る。:math:`\lambda_1,\lambda_2` は共分散行列の固有値である。

二次元の正規分布では、:math:`1\sigma` 楕円の内側に入る確率は :math:`1-e^{-1/2}\approx39\ \%`、:math:`3\sigma` 楕円では :math:`1-e^{-9/2}\approx99\ \%` である :cite:`johnson`。
一次元の「:math:`1\sigma` で 68 %」とは異なるので注意してほしい。
この統計量は、着地点が（二次元の）正規分布に従うという仮定の下での要約である。実際の分布は歪むことがあるので、
最大着地距離（``max_landing_distance``）とそのサンプル番号も併せて出力する。頂点高度の平均（:math:`n\ge1`）と標準偏差（:math:`n-1` で割る。:math:`n\ge2`）も求める。

.. _sec-mc-percentile:

着地距離のパーセンタイル
~~~~~~~~~~~~~~~~~~~~~~~~

着地距離の分布の裾は、パーセンタイル（P50、P90、P99、P99.9）でも報告する。着地したサンプルの着地距離を昇順に並べた :math:`d_{(0)}\le\dots\le d_{(n-1)}` から、:math:`q` パーセンタイルを線形補間

.. math::
   :label: eq-mc-percentile

   d_q = d_{(i)} + (d_{(i+1)}-d_{(i)})\,f,\qquad x=\frac{q}{100}(n-1),\quad i=\lfloor x\rfloor,\quad f=x-i

で求める（:math:`n=1` ならその値、:math:`n=0` なら NaN）。
P99.9 は、着地したサンプルが 1000 以上の降下モードだけで求める。
それより少ないと、P99.9 は最大のサンプルかそのすぐ隣との補間で、上位 0.1 % の裾の推定にならないからである。
1000 未満では ``dispersion_summary.json`` の ``landing_distance_p999`` を ``null`` とし、端末には ``P99.9 n/a (needs ≥ 1000 landed samples)``\ （結果パネルでは ``n/a (needs 1000+)``）と表示する。
誤差楕円は着地点の分布を二次元の正規分布とみなした要約なので、P90、P99、P99.9 は、保安域の半径を考えるときに分布の形によらない目安になる。
パーセンタイルも、失敗したサンプルを含まないので、``complete`` が ``false`` のときは過小になりうる。

サンプル数と最大値
~~~~~~~~~~~~~~~~~~

モンテカルロ解析の最大着地距離は、サンプル数とともに大きくなる。
ばらつきは正規分布で、その裾には上限がないので、サンプルを増やすほど極端な値（たとえば標準偏差の 4〜5 倍の軸力の減少）を引くからである。
相対的なばらつき（``thrust_scale``、``cn_scale``、``ca_scale`` などの倍率）も、既定では裾を切らない正規分布から引く。倍率には 0.05 の下限（:eq:`eq-mc-factor`）があるだけなので、
``ca_scale`` の :math:`-0.72`\ （標準偏差 0.15 の 4.8 倍、軸力が 0.28 倍）のような、現実には起こりにくい値も引かれ、それが最大値を決めることがある。
最大値を決めているサンプルのばらつきは、下の 3 つ目の項目のように確かめる。
各サンプルの値は ``seed`` とサンプル番号だけで決まるので（:ref:`sec-mc`）、サンプル数を増やした計算は少ない計算のサンプルをすべて含み、最大値は減ることがない。
一方、P99 などのパーセンタイルはサンプル数とともに一定の値に収まる。
:numref:`tbl-safety-samples` に、サンプル（``power`` の風）について、:ref:`sec-dispersion-safety` の手順 3 の 2 つのケースで、サンプル数を変えた結果を示す。P99.9 は ``dispersion_summary.json`` の ``landing_distance_p999`` である。

.. _tbl-safety-samples:

.. list-table:: サンプル数と着地距離 [m]（サンプル、``power`` の風、手順 3 の機体のばらつきだけ、``seed = 1``。弾道落下はパネル法で 7 m/s、270°、パラシュート降下（表の「パラシュート」）は部品積み上げ法で 7 m/s、90°）
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
     - 2626.7
     - 3001.9
     - 3015.3
     - 20267.9
     - 24167.2
     - 28927.8
   * - :math:`10^4`
     - 2567.9
     - 2992.4
     - 3935.2
     - 20152.8
     - 23086.8
     - 28927.8
   * - :math:`4\times10^4`
     - 2584.8
     - 3101.5
     - 3935.2
     - 20104.7
     - 23240.3
     - 28927.8
   * - :math:`10^5`
     - 2595.9
     - 3129.1
     - 5296.8
     - 20183.0
     - 23271.7
     - 36278.6

P99 と P99.9 は :math:`10^3` から :math:`10^5` サンプルで数 % しか変わらないが、最大値は :math:`10^5` サンプルで大きく増える。
:math:`10^5` サンプルの最大は、どちらの降下モードも、軸力の倍率が 0.28（``ca_scale`` の :math:`-0.72`、標準偏差 0.15 の 4.8 倍）で頂点が 10937 m（弾道落下のパネル法では 9404 m）まで上がった 1 飛翔で決まる。
同じ計算を ``truncate_sigma = 3`` で行うと、この飛翔の ``ca_scale`` は :math:`-0.45`\ （3 倍）に切られ、最大は :math:`10^4` サンプルと同じ 28927.8 m になる。P99 は 20151.6 m、P99.9 は 23009.8 m で、切らない計算とほとんど変わらない。
サンプル数と切り詰めの使い方は、上の「サンプル数」と「乱数の切り詰め」の判断の目安に従う。最大値を決めている飛翔（``max_landing_distance_sample`` の行）のばらつきは ``dispersion_mc.csv`` で見て、その組合せが起こりうるかを判断する。

最悪の重ね合わせ
~~~~~~~~~~~~~~~~

手順 3 と 5 は、上限の風、機体のばらつきの裾、回収系の故障という、それぞれ起こりにくい条件を重ねる。
同時に起こる確率は小さく、得られる半径は意図して保守的なものである。どの条件までを重ねるか（たとえば上限の風とメインが頂点で開く故障を同時に考えるか）は、打上げを行う団体が決める。
重ねた結果の半径は、頂点高度の数倍になりうる（:ref:`sec-dispersion-safety` の計算例では、頂点高度約 4.4 km に対してパラシュート降下の最大が 28.9 km）。

適用範囲と限界
--------------

全体の一覧は :ref:`sec-lim-uncertainty` にある。

* ばらつきは互いに独立な正規分布で、項目どうしの相関、別の分布、機体形状そのもののばらつきは扱わない。
* 空力係数のばらつきは、表の値に一律の倍率を掛けるだけで、Mach 数や迎角による誤差の違いは表せない。
* 風は定常である。風の格子とモンテカルロ解析の最大値は、計算した条件の中の最大である。
* 着地しない飛翔や失敗したサンプルは統計に入らない。1 件でもあると統計は不完全で、最大着地距離は下限になる（:ref:`sec-mc-failure`）。
* 静的に不安定なケースと横向きに落ちたケースは、統計に残して数える（:ref:`sec-dispersion-unstable`、:ref:`sec-dispersion-broadside`）。
* 誤差楕円は着地点を正規分布とみなした要約である。歪む分布には、最大着地距離とパーセンタイルを使う。

精度の根拠
----------

* **サンプル数の収束**：P99 と P99.9 は :math:`10^3` から :math:`10^5` サンプルで数 % しか変わらない（:numref:`tbl-safety-samples`）。この比較は同じモデルでサンプル数だけを変えたもので、統計量の数値的な収束を示す。
* **誤差楕円の仮定**：1000 サンプルの例（サンプル、``seed = 1``）で、1σ と 3σ の楕円の内側に入る点の割合は、弾道落下で 44 % と 98 %、パラシュート降下で 41 % と 99 % である。
  二次元の正規分布の 39 % と 99 % とわずかに違うので、実際の分布は正規分布から少しずれている（:ref:`page-howto-dispersion-run`）。
* **飛翔モデル**：1 回の飛翔の検証は :ref:`page-theory-dynamics`、:ref:`page-theory-descent`、:ref:`page-theory-integrators` の「精度の根拠」にある。
* 落下分散の統計量を実測の落下点と比べた妥当性確認はない。結果はモデルの結果である。

関連する設定キー
----------------

.. list-table::
   :header-rows: 1
   :widths: 36 16 48

   * - キー
     - 既定値
     - 効果の方向
   * - :ref:`dispersion.mode <cfg-dispersion-mode>`
     - ``"wind_grid"``
     - ``"monte_carlo"`` でばらつきを含む分布を求める。
   * - :ref:`dispersion.wind_speeds <cfg-dispersion-wind_speeds>`
     - 1〜7 m/s
     - 刻みを細かくすると最大値が収束する。
   * - :ref:`dispersion.directions <cfg-dispersion-directions>`
     - 8
     - 増やすと多角形が滑らかになる。36 以上にする。
   * - :ref:`dispersion.monte_carlo.samples <cfg-dispersion-monte_carlo-samples>`
     - 1000
     - 増やすと P99.9 が安定し、最大値が増える。:math:`10^4` 以上を使う。
   * - :ref:`dispersion.monte_carlo.seed <cfg-dispersion-monte_carlo-seed>`
     - 1
     - 同じ値なら同じ結果になる。
   * - :ref:`dispersion.monte_carlo.truncate_sigma <cfg-dispersion-monte_carlo-truncate_sigma>`
     - 未設定（切らない）
     - 設定すると遠い裾の最大値が小さくなる。P99、P99.9 はほとんど変わらない。
   * - :ref:`dispersion.monte_carlo.* <cfg-dispersion-monte_carlo-thrust_scale>`
     - :numref:`tbl-mc-perturb`
     - 標準偏差を大きくするほど分布が広がる。
