.. _page-theory-cfd-equations:

CFD の方程式と離散化
======================

本章は、CFD が解く方程式（Euler 方程式と RANS 方程式）、境界条件、基準量、数値解法を述べる。
どの方程式を解くかは ``model``、対流項の離散化は ``scheme`` で選ぶ。メッシュと壁面の決め方は :ref:`page-theory-cfd-choose`、解の係数への変換は :ref:`page-theory-cfd-coefficients` にある。

この章で決めること
--------------------

.. list-table::
   :header-rows: 1
   :widths: 24 30 46

   * - 決めること
     - 設定キー
     - 本章の節
   * - 解く方程式
     - :ref:`model <cfg-aero-cfd-model>`
     - :ref:`sec-cfd-euler-eq`、:ref:`sec-cfd-rans-eq`
   * - 対流項の離散化
     - :ref:`scheme <cfg-aero-cfd-scheme>`
     - :ref:`sec-cfd-int-discretization`
   * - 時間積分の速さ、反復の上限、残差の目標
     - :ref:`cfl <cfg-aero-cfd-cfl>`、:ref:`iterations <cfg-aero-cfd-iterations>`、:ref:`convergence <cfg-aero-cfd-convergence>`、:ref:`multigrid <cfg-aero-cfd-multigrid>`
     - :ref:`sec-cfd-int-discretization`
   * - 壁面の扱い（壁面を解像するか壁関数か）
     - :ref:`yplus <cfg-aero-cfd-yplus>`
     - :ref:`sec-cfd-rans-eq`
   * - ロール減衰の回転の速さ
     - :ref:`roll_rate_hat <cfg-aero-cfd-roll_rate_hat>`
     - :ref:`sec-cfd-sideslip`

選び方
--------

方程式は ``model``、離散化は ``scheme`` で選ぶ。``model`` の選び方は :ref:`sec-cfd-choose-model` の流れ図と表にある。ここでは ``scheme`` と、解の速さに関わる設定の選び方を示す。

.. _tbl-choose-cfd-scheme:

.. list-table:: 対流項の離散化（``scheme``）
   :header-rows: 1
   :widths: 18 41 41

   * - 項目
     - ``"roe"``\ （既定）
     - ``"jst"``
   * - 方法
     - Roe の近似 Riemann 解法 :cite:`roe` に、MUSCL 形の再構成と制限関数を組み合わせた 2 次精度
     - Jameson–Schmidt–Turkel の中心差分と人工粘性 :cite:`jst`。再構成は使わない
   * - 衝撃波の扱い
     - 風上化により、衝撃波を数セルで捉える
     - 人工粘性の係数に依存する
   * - 使える流れのモデル
     - Euler と RANS
     - Euler の比較用。結果の精度は検証していない
   * - 選ぶとき
     - 通常（検証したのはこちら）
     - 離散化の違いによる結果の変化を調べたいとき
   * - 避けるとき
     - なし
     - 係数表を作るとき（:math:`\alpha=0` の見かけの法線力が Roe と異なる符号で出た例がある。:ref:`sec-cfd-val-asym-cause`）

``cfl``、``iterations``、``convergence``、``multigrid`` は、解く速さと反復の打ち切りを決める。
既定は、サンプル機体で早期終了の基準との係数の差が 0.06 % 以内に収まるように測って決めてあり（:ref:`sec-cfd-early-stop`）、変える理由は通常ない。
``multigrid`` は、サンプルのメッシュでは SU2 が粗い格子を作れないので、設定しても結果も反復数も変わらない（:ref:`sec-cfd-multigrid`）。
ケースが ``unconverged`` や失敗になるときの対処は :ref:`sec-cfd-int-convergence` にある。

モデル
--------

.. _sec-cfd-int-roles:

使うツール
~~~~~~~~~~

CFD モードは、流れの解法に `SU2 <https://su2code.github.io/>`__ :cite:`su2`、体積メッシュの生成に `gmsh <https://gmsh.info/>`__ :cite:`gmsh` を使う。
CAD のモデル（``surface = "step"``）や STL（``surface = "stl"``）を壁面にするときは、OpenCASCADE :cite:`occt` と Manifold :cite:`manifold` を含むツールも要る。1 ケースを複数のランクで解くときは Open MPI :cite:`openmpi` を使う。
必要なツールと用意の手順は :ref:`page-howto-cfd-setup` にある。

.. _sec-cfd-euler-eq:

Euler 方程式
~~~~~~~~~~~~

非粘性の圧縮性流れは、質量、運動量、エネルギーの保存則で表される。検査体積について保存形で書くと

.. math::
   :label: eq-cfd-euler

   \frac{\partial}{\partial t}\int_\Omega \bm{U}\,\dd\Omega + \oint_{\partial\Omega}\bm{F}\cdot\bm{n}\,\dd S = 0,\qquad
   \bm{U}=\begin{pmatrix}\rho\\ \rho\bm{u}\\ \rho E\end{pmatrix},\quad
   \bm{F}=\begin{pmatrix}\rho\bm{u}\\ \rho\bm{u}\otimes\bm{u}+p\,\mathsf{I}\\ (\rho E+p)\bm{u}\end{pmatrix}

である。:math:`\Omega` は検査体積、:math:`\rho` は密度、:math:`\bm{u}` は速度、:math:`E` は単位質量あたりの全エネルギー、:math:`p` は圧力である。
理想気体として :math:`p=(\gamma-1)\left(\rho E-\tfrac12\rho|\bm{u}|^2\right)` で閉じ、比熱比 :math:`\gamma` と気体定数 :math:`R` は大気モデルの値（空気）を使う。
壁面には滑りの境界条件（流れが壁を通り抜けない条件）を課す。Euler 方程式には粘性がないので、壁面の力は圧力だけで決まり、摩擦抗力は含まれない。

.. _sec-cfd-rans-eq:

RANS と Spalart–Allmaras モデル
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

RANS（Reynolds 平均 Navier–Stokes 方程式）は、:eq:`eq-cfd-euler` に粘性応力と熱伝導の項を加え、乱流の効果を渦粘性として足したものである。
粘性係数は Sutherland の式（:math:`\mu_{ref}=1.716\times10^{-5}` Pa s、:math:`T_{ref}=273.15` K、定数 110.4 K）で与え、大気モデル（:ref:`page-theory-atmosphere-gravity`）の Sutherland 則に一致する。
渦粘性は一方程式モデルの Spalart–Allmaras（SA）モデル :cite:`spalart1992,spalart1994` で求める。修正した渦粘性 :math:`\tilde\nu` の輸送方程式は

.. math::
   :label: eq-cfd-sa

   \frac{\partial\tilde\nu}{\partial t}+\bm{u}\cdot\nabla\tilde\nu
   = c_{b1}\tilde S\,\tilde\nu-c_{w1}f_w\left(\frac{\tilde\nu}{d}\right)^2
   +\frac{1}{\sigma}\left[\nabla\cdot\bigl((\nu+\tilde\nu)\nabla\tilde\nu\bigr)+c_{b2}|\nabla\tilde\nu|^2\right]

で、渦粘性は :math:`\mu_t=\rho\tilde\nu f_{v1}` となる。:math:`d` は最寄りの壁までの距離、:math:`\tilde S` は渦度に基づく生成項、
:math:`c_{b1},c_{b2},c_{w1},\sigma` はモデル定数、:math:`f_{v1},f_w` は壁近傍の減衰関数である。
:math:`\tilde\nu` が負になったときにも解が壊れない負の値の扱い（SA-neg）と、遷移の項 :math:`f_{t2}` を付けて使う。自由流の渦粘性は :math:`\tilde\nu=3\nu` である。

壁面（``wall`` と ``base``）は断熱の粘着壁（熱流束 0）である。壁面の扱いは ``yplus`` で選ぶ。

* ``yplus`` が 5 以下（既定は 1）：最初の層を粘性底層に置いて壁面まで境界層を解像し、壁関数は使わない。
* ``yplus`` が 30 から 150：対数則に基づく標準の壁関数を ``wall`` と ``base`` に使う。検証していない。
* その間の値、または 150 より大きい値：どちらの扱いも成り立たないのでエラーにする。

最初の層の高さの決め方は :ref:`sec-cfd-prism` にある。

.. _sec-cfd-bc:

境界条件と基準量
~~~~~~~~~~~~~~~~

境界は次の名前で区別する（:numref:`fig-cfd-bc`）。

.. _fig-cfd-bc:

.. figure:: /_generated/tikz/cfd_bc.*
   :width: 100%

   対称面 :math:`y=0` での CFD の座標、自由流、境界の名前、基準量。:math:`x` は後方、:math:`z` は上向きで、モーメントはノーズ先端のまわりにとる。見るところは、力を積分する面が wall だけで、底面（base）は別の名前で区別されることと、自由流を傾けて迎角を与えることである。

.. list-table::
   :header-rows: 1
   :widths: 20 80

   * - 境界
     - 条件
   * - ``wall``
     - 機体表面（胴体とフィン）。力を積分する面。Euler では滑り壁、RANS では断熱の粘着壁
   * - ``base``
     - 底面。Euler ではフェアリングの面（滑り壁）で、力を積分しない。RANS では平らな底面の断熱の粘着壁で、力は軸力に使うかを ``rans_base`` で選ぶ（:ref:`sec-cfd-rans-use`）
   * - ``farfield``
     - 遠方境界。特性量に基づく非反射の境界条件
   * - ``symmetry``
     - :math:`y=0` の平面。半領域のときだけ。流れの法線方向の成分がない対称の境界。RANS では、プリズム層の側面も含む

自由流は、射点の標高（``launch.altitude``）の大気の圧力と温度の理想気体で、Mach 数 :math:`M` と迎角 :math:`\alpha`、横滑り角 :math:`\beta` の向きの一様流である。
機体はメッシュの中に固定し、自由流の向きを傾けるので、SU2 の力とモーメントの係数がそのまま機体軸の係数になる。
座標は、:math:`x` がノーズ先端から後方への距離、:math:`z` が上向き、:math:`y` が横方向である（CFD の座標。機体座標との関係は :ref:`sec-cfd-body6`）。
自由流は :math:`\alpha>0` で :math:`+z` 方向の成分を持つので、機体には :math:`+z` 向きの法線力が働く。
モーメントはノーズ先端のまわりにとり、基準長さは機体の直径 :math:`d`、基準面積は断面積 :math:`S` である。半領域では基準面積を :math:`S/2` にするので、半モデルの力がそのまま全機の係数になる。全領域では :math:`S` である。

.. _sec-cfd-int-discretization:

離散化
~~~~~~

SU2 は非構造メッシュの節点を中心とする有限体積法で、各節点のまわりの双対セルについて :eq:`eq-cfd-euler` を離散化する。選べる設定は次のとおりである。

* **対流フラックス**\ ：``scheme = "roe"``\ （既定）は Roe の近似 Riemann 解法 :cite:`roe`、``"jst"`` は Jameson–Schmidt–Turkel の中心差分と人工粘性 :cite:`jst` である（:numref:`tbl-choose-cfd-scheme`）。
* **2 次精度化**\ ：Roe では、MUSCL 形の再構成に制限関数を組み合わせた 2 次精度を使う。制限関数は ``model`` ごとに決まっていて、設定では選べない。JST は再構成を使わない。
* **時間積分**\ ：定常解を得るために、陰的 Euler 法による擬似時間の進行を使う。CFL 数は適応的に変え、既定の初期値 ``cfl`` = 5 から、収束が進むと増やし、悪化すると減らす。
* **収束判定**\ ：Euler は、密度の二乗平均残差が ``convergence``\ （既定 :math:`10^{-6}`）以下になるか、``iterations``\ （既定 3000）に達すると終了する。実際に止めるかどうかは係数の落ち着きも見て決める（:ref:`sec-cfd-int-convergence`）。
  RANS は、密度と SA の残差が基準の最大値から十分に下がり、係数が平らになったときに止める（:ref:`sec-cfd-int-rans-accept`）。反復数の上限は ``iterations``\ （RANS の既定 4000）で、2 次精度の段で数える。
* **RANS の対流**\ ：乱流量の対流は 1 次風上である。冷えた起動のケースは、1 次精度で解いた解から 2 次精度で解き直す（:ref:`sec-cfd-int-rans-stages`）。
* **マルチグリッド**\ ：``multigrid``\ （既定 0）を 1 から 4 にすると SU2 の凝集型マルチグリッドを要求するが、サンプルのメッシュでは粗い格子が作られず、効果がない（:ref:`sec-cfd-multigrid`）。

.. _sec-cfd-sideslip:

横滑り角とロール角速度
~~~~~~~~~~~~~~~~~~~~~~

機体座標の対気速度 :math:`(u,v,w)` から、迎角と横滑り角は :math:`\alpha=\operatorname{atan2}(w,u)`、:math:`\beta=\arcsin(v/V)` と定義する（:eq:`eq-alpha-beta`）。
SU2 の自由流の向きは CFD の座標で :math:`(\cos\alpha\cos\beta',\ \sin\beta',\ \sin\alpha\cos\beta')` で、その横滑り角 :math:`\beta'` は機体軸の :math:`\beta` と符号が逆である（:math:`\beta'=-\beta`）。
したがって SU2 には :math:`\alpha` と :math:`-\beta` を渡す。この符号は、軸対称な表の展開（:eq:`eq-coeff-6-axisym`）との一致で確かめた（:ref:`ロールと符号の検証 <sec-cfd-val-roll>`）。

ロール減衰微係数のケースは、機体を回転させずに、定常な回転座標系で解く。
:math:`\alpha=\beta=0` で、機体がロール角速度 :math:`p` で回る流れを与える。回転の中心はノーズ先端の原点で、軸は機軸（CFD の :math:`x`）である。
CFD の :math:`x` は後ろ向きなので、機体軸の正のロール角速度は :math:`-x` まわりの回転になり、符号が逆になる。
角速度は無次元のロール角速度 :math:`\hat p` から固定した式

.. math::
   :label: eq-cfd-roll-rate

   p=\frac{2V}{d}\,\hat p,\qquad V=M\,a

で決める。:math:`a` は基準高度（``aero.reference_altitude``）の音速、:math:`\hat p` は ``roll_rate_hat``\ （既定 0.01）である。
Mach 数ごとに :math:`V` を決めるので、:math:`C_{lp}=\partial C_l/\partial\hat p` は飛翔計算の正規化（:eq:`eq-cl-total`）と同じ意味を持つ。

適用範囲と限界
----------------

* **非粘性**\ ：Euler は摩擦とはく離を含まない。底面の後流は解が定まらないので、尾部フェアリングで避ける（:ref:`sec-cfd-fairing`）。亜音速の軸力の理論値は 0（d'Alembert のパラドックス）で、実際の値は小さな負の数になる（サンプル機体の迎角 0〜8° で :math:`-0.01` から :math:`-0.05`）。
* **乱流モデル**\ ：SA は一方程式の渦粘性モデルで、付着した流れの摩擦を対象に検証した。底面の後流の圧力、はく離の大きい流れは確かめていない。壁関数（``yplus`` 30〜150）は検証していない。
* **定常解**\ ：どちらのモデルも定常解を (M, α, β) ごとに求める。非定常な空力（ピッチ減衰など）は直接得られない（:ref:`sec-cfd-supplied`）。
* **ロール減衰**\ ：回転座標系の Euler の解で、粘性の寄与（フィンの摩擦）は含まない。RANS では解かない。
* **大気**\ ：自由流は理想気体の空気で、射点の標高の圧力と温度を使う。一様流で、風のせん断は含まない。

精度の証拠
------------

* **Euler**\ ：半頂角 10° の円すいの表面圧力を :math:`M=2`、:math:`\alpha=0` で Taylor–Maccoll の解 :cite:`taylormaccoll` と比べると、``wall_size`` 10、5、2.5 mm で誤差は 11.2、5.4、1.8 % と減る（:ref:`sec-vaero-cfd`）。これは検証である。
* **RANS**\ ：平板の摩擦係数は、:math:`M=0.2` で NASA TMR の SA の解と 1 % 以内、:math:`M=2` で van Driest II の相関式と 1.5 % 以内で一致する（:ref:`sec-cfd-rans`）。これも検証で、実測との比較（妥当性確認）ではない。
* **符号と回転座標系**\ ：横滑りとロール減衰の符号と無次元化は、軸対称表を展開した値と一致する（:ref:`sec-cfd-val-roll`）。
* **離散化の設定**\ ：早期終了とマルチグリッドの測定は :ref:`sec-cfd-early-stop` と :ref:`sec-cfd-multigrid` にある。

関連する設定キー
------------------

値の完全な定義は :ref:`page-reference-config-cfd` にある。

.. list-table::
   :header-rows: 1
   :widths: 22 22 56

   * - キー
     - 既定値
     - 効果の方向
   * - :ref:`model <cfg-aero-cfd-model>`
     - ``"euler"``
     - ``"rans"`` で粘性と乱流を含める
   * - :ref:`scheme <cfg-aero-cfd-scheme>`
     - ``"roe"``
     - ``"jst"`` は中心差分と人工粘性（検証していない）
   * - :ref:`cfl <cfg-aero-cfd-cfl>`
     - 5
     - 大きくすると速いが、発散しやすくなる
   * - :ref:`iterations <cfg-aero-cfd-iterations>`
     - 3000（RANS は 4000）
     - 反復の上限
   * - :ref:`convergence <cfg-aero-cfd-convergence>`
     - :math:`10^{-6}`
     - Euler の密度の残差の目標
   * - :ref:`multigrid <cfg-aero-cfd-multigrid>`
     - 0
     - 1〜4 を指定しても、サンプルのメッシュでは効果がない
   * - :ref:`yplus <cfg-aero-cfd-yplus>`
     - 1
     - 5 以下は壁面を解像、30〜150 は壁関数
   * - :ref:`roll_rate_hat <cfg-aero-cfd-roll_rate_hat>`
     - 0.01
     - ロール減衰のケースの無次元ロール角速度
