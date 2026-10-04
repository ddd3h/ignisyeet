CFD による空力解析
==================

前章までの部品積み上げ法（:doc:`barrowman`）とパネル法（:doc:`panel`）は、どちらも機体のまわりの流れ場を解かずに係数を求める。
本章では、機体のまわりの空間を格子で覆い、圧縮性流れの方程式を数値的に解く **CFD**\ （計算流体力学）を説明する。
設定ファイルで ``aero.method = "cfd"`` を選ぶと、他の手法と同じ形式の係数表（:ref:`sec-aero-table`）が CFD の結果から作られ、
飛翔計算は何も変えずにそれを使う。

流れの解法にはオープンソースの `SU2 <https://su2code.github.io/>`__ :cite:`su2`、メッシュの生成には `gmsh <https://gmsh.info/>`__ :cite:`gmsh` と `OpenCASCADE <https://dev.opencascade.org/>`__ :cite:`occt` を使う。
STL をそのまま解くときは、重なった閉じた殻の和に `Manifold <https://github.com/elalish/manifold>`__ :cite:`manifold` を使う。
IgnisYeet はこれらを外部プログラムとして呼び出し、形状の再構成（または STL の修復と和）、メッシュ、(Mach 数, 迎角, 横滑り角) の各ケースの実行、結果の後処理、係数表の作成までを自動で行う。

位置付け
--------

いつ CFD を使うか
~~~~~~~~~~~~~~~~~

三つの手法の違いと計算量は :ref:`sec-aero-methods` にまとめた。

CFD は、衝撃波や遷音速の流れ、フィンと胴体の干渉を物理方程式から直接得られる。
STL をそのまま解くこともでき、横滑りとロールを含む 6 成分の係数表（:ref:`sec-aero-coeff6`）を作れる。
一方で、1 ケースごとに 10\ :sup:`5` から 10\ :sup:`6` 個の格子点の流れ場を求めるため、計算には数十分から数時間かかり、メモリも使う（:ref:`sec-cfd-run` に目安を示す）。
そこで使いどころは、部品積み上げ法やパネル法の結果を独立な方法で確かめたいとき、あるいは遷音速・超音速の係数をより信頼できる値にしたいときである。
CFD が出す係数表は、他の手法の表と同じ形式なので、飛翔計算の側は区別しない。

本実装の既定は非粘性の Euler 方程式である。粘性による摩擦抗力、底面抗力、フィン縁の抗力は、部品積み上げ法の項で補う（:ref:`sec-cfd-post`）。
乱流を含む RANS も選べるが、現状は検証できておらず、信頼できる結果を出せる段階にはない（:ref:`sec-cfd-rans`）。

必要なツール
~~~~~~~~~~~~

CFD モードには次の外部プログラムが必要である。

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - プログラム
     - 用途
   * - SU2_CFD（8.3.0）
     - 流れの解法。Euler 方程式と RANS 方程式を解く :cite:`su2`
   * - gmsh（Python API）
     - 体積メッシュの生成。形状の再構成には OpenCASCADE のカーネルを使う :cite:`gmsh,occt`
   * - Open MPI（``mpirun``）
     - 1 ケースを複数のランクで並列に解くとき（``ranks_per_case`` が 2 以上のとき）に必要 :cite:`openmpi`
   * - manifold3d（Python）
     - STL の閉じた殻の和（ブーリアン演算）。``surface = "stl"`` のときだけ必要 :cite:`manifold`

これらは conda-forge のパッケージとして入手でき、micromamba で一つの環境にまとめて入れるのが簡単である。
例えば次のようにする。

.. code-block:: sh

   micromamba create -n ignisyeet-cfd -c conda-forge su2=8.3.0 python-gmsh openmpi

SU2 のバージョンは 8.3.0 に固定している。8.5.0 は AVX-512 命令を持たない CPU で異常終了することが確かめられたためである。

manifold3d は、同じ環境の Python に pip で入れる。

.. code-block:: sh

   <prefix>/bin/python -m pip install manifold3d

インストーラ（``install.sh --cfd``）は、環境を作ったあとにこれを実行する。
``cfd-check`` が manifold3d を検査するのは ``surface = "stl"`` のときだけで、それ以外では入っていなくてもよい。

IgnisYeet は、設定の ``prefix`` で指す環境の ``bin`` にある実行ファイルを探す。
``prefix`` が空のときは環境変数 ``IGNISYEET_CFD_PREFIX`` を使い、それも空のときは ``PATH`` から探す。
子プロセスには ``<prefix>/bin`` を ``PATH`` の先頭に、``<prefix>/lib`` を ``LD_LIBRARY_PATH`` に加えて渡す。
設定の先頭の ``~`` はホームディレクトリに展開される。

準備ができているかは、次のコマンドで確かめる。

.. code-block:: sh

   ignisyeet cfd-check examples/sample_cfd.toml
   ignisyeet cfd-check examples/sample_cfd.toml --mesh

最初のコマンドは、SU2、MPI、gmsh（``surface = "stl"`` のときは manifold3d も）の場所とバージョンを表示し、設定の計算ケース数とともに「CFD mode is ready」または不足しているツールを報告する。
``--mesh`` を付けると、実際にメッシュを作り、節点数、セル数、壁面の三角形数、セルの種類、品質（SICN の最小値と平均、0.05 未満のセル数）を表示する。
``surface = "stl"`` では、形状の忠実度（:ref:`sec-cfd-fidelity`）も表示する。
メッシュはキャッシュされるので、本番の計算でそのまま再利用される。
CFD モードで ``aero`` を実行したときも、先に同じ検査を行い、ツールが足りなければ計算を始める前にエラーで止まる。

支配方程式と数値解法
--------------------

Euler 方程式
~~~~~~~~~~~~

非粘性の圧縮性流れは、質量、運動量、エネルギーの保存則で表される。検査体積 :math:`\Omega` について保存形で書くと

.. math::
   :label: eq-cfd-euler

   \frac{\partial}{\partial t}\int_\Omega \bm{U}\,\dd\Omega + \oint_{\partial\Omega}\bm{F}\cdot\bm{n}\,\dd S = 0,\qquad
   \bm{U}=\begin{pmatrix}\rho\\ \rho\bm{u}\\ \rho E\end{pmatrix},\quad
   \bm{F}=\begin{pmatrix}\rho\bm{u}\\ \rho\bm{u}\otimes\bm{u}+p\,\mathsf{I}\\ (\rho E+p)\bm{u}\end{pmatrix}

である。:math:`\rho` は密度、:math:`\bm{u}` は速度、:math:`E` は単位質量あたりの全エネルギー、:math:`p` は圧力である。
理想気体として :math:`p=(\gamma-1)\left(\rho E-\tfrac12\rho|\bm{u}|^2\right)` で閉じ、比熱比 :math:`\gamma` と気体定数 :math:`R` は大気モデルの値（空気）を使う。

壁面（``wall``）には滑りの境界条件（流れが壁を通り抜けない条件）を課す。
Euler 方程式には粘性がないので、壁面の力は圧力だけで決まり、摩擦抗力は含まれない。

RANS と Spalart–Allmaras モデル
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

RANS（Reynolds 平均 Navier–Stokes 方程式）は、:eq:`eq-cfd-euler` に粘性応力と熱伝導の項を加え、乱流の効果を渦粘性として足したものである。
粘性係数は Sutherland の式（:math:`\mu_{ref}=1.716\times10^{-5}` Pa s、:math:`T_{ref}=273.15` K、定数 110.4 K）で与え、大気モデル（:doc:`environment`）の Sutherland 則に一致する。
渦粘性は一方程式モデルの Spalart–Allmaras（SA）モデル :cite:`spalart1992,spalart1994` で求める。
修正した渦粘性 :math:`\tilde\nu` の輸送方程式は

.. math::
   :label: eq-cfd-sa

   \frac{\partial\tilde\nu}{\partial t}+\bm{u}\cdot\nabla\tilde\nu
   = c_{b1}\tilde S\,\tilde\nu-c_{w1}f_w\left(\frac{\tilde\nu}{d}\right)^2
   +\frac{1}{\sigma}\left[\nabla\cdot\bigl((\nu+\tilde\nu)\nabla\tilde\nu\bigr)+c_{b2}|\nabla\tilde\nu|^2\right]

で、渦粘性は :math:`\mu_t=\rho\tilde\nu f_{v1}` となる。:math:`d` は最寄りの壁までの距離、:math:`\tilde S` は渦度に基づく生成項、
:math:`c_{b1},c_{b2},c_{w1},\sigma` はモデル定数、:math:`f_{v1},f_w` は壁近傍の減衰関数であり、SU2 の標準の SA モデルをそのまま使う。
壁面は断熱の粘着壁とし、壁面の近くの速度分布には対数則に基づく壁関数（SU2 の ``STANDARD_WALL_FUNCTION``）を使う。
自由流の渦粘性は :math:`\tilde\nu=3\nu`\ （``FREESTREAM_NU_FACTOR = 3``）である。

離散化
~~~~~~

SU2 は非構造メッシュの節点を中心とする有限体積法で、各節点のまわりの双対セルについて\ :eq:`eq-cfd-euler` を離散化する。
IgnisYeet が設定する数値解法は次のとおりである。

* **対流フラックス**\ ：既定は Roe の近似 Riemann 解法 :cite:`roe`\ 。``scheme = "jst"`` では Jameson–Schmidt–Turkel の中心差分と人工粘性 :cite:`jst`\ （センサ係数 0.5 と 0.02）を使う。
* **2 次精度化**\ ：Roe のときは MUSCL 形の再構成（勾配は重み付き最小二乗法）に Venkatakrishnan の制限関数 :cite:`venkat`\ （係数 0.05）を組み合わせる。
  ``min_iterations``\ （既定 100）反復の時点で制限関数の値を固定する。固定しないと、制限関数の切り替わりで残差が 10\ :sup:`-4` 付近から下がらなくなるためである。
  固定の反復は早期終了の最小反復数（:ref:`sec-cfd-run`）と同じにしてあり、残差が下がる余地を残して止める。
  JST では再構成を使わない。
* **時間積分**\ ：定常解を得るために、陰的 Euler 法による擬似時間の進行を使う。連立 1 次方程式は FGMRES（前処理は ILU、許容 10\ :sup:`-6`、10 反復）で解く。
  CFL 数は適応的に変え、既定の初期値 ``cfl`` = 5 から、収束が進むと 1.2 倍に増やし、悪化すると 0.5 倍に減らす。範囲は :math:`0.2\,\mathrm{cfl}` から :math:`50\,\mathrm{cfl}` である。
* **収束判定**\ ：密度の二乗平均残差が ``convergence``\ （既定 :math:`10^{-6}`）以下になるか、``iterations``\ （既定 3000）に達すると終了する。判定は 10 反復目から行う。
* **マルチグリッド**\ ：``multigrid``\ （``MGLEVEL``、既定 0）を 1 から 4 にすると、SU2 の凝集型マルチグリッドの設定（``MGLEVEL``、``MGCYCLE``）を書く。
  サンプルのメッシュでは SU2 が粗い格子を作れないので、効果がない（:ref:`sec-cfd-multigrid`）。

RANS では、乱流量の対流は 1 次風上、時間積分は陰的 Euler 法とし、流れの対流は **1 次精度** の Roe（再構成なし）を使う。
接線方向に細長い四面体の壁面メッシュで 2 次の再構成が発散したためである。
CFL の上限は :math:`4\,\mathrm{cfl}` である。残差は 10\ :sup:`-5` 付近で止まり、数百反復のあと発散に向かう一方、係数は 100 反復ごろから平らになるので、反復数の上限を 300 にしている。

実際に生成される SU2 の設定ファイル（Euler、Roe、:math:`M=2`、:math:`\alpha=4^\circ`、半モデル、基準直径 0.1 m）の主な行を示す。

.. code-block:: text

   SOLVER= EULER
   MACH_NUMBER= 2.000000
   AOA= 4.000000
   SIDESLIP_ANGLE= 0.0
   FLUID_MODEL= IDEAL_GAS
   INIT_OPTION= TD_CONDITIONS
   FREESTREAM_OPTION= TEMPERATURE_FS
   FREESTREAM_PRESSURE= 101325.0000
   FREESTREAM_TEMPERATURE= 288.1500
   REF_ORIGIN_MOMENT_X= 0.0
   REF_LENGTH= 0.10000000
   REF_AREA= 0.0039269908
   MARKER_EULER= ( wall, base )
   MARKER_MONITORING= ( wall )
   MARKER_FAR= ( farfield )
   MARKER_SYM= ( symmetry )
   NUM_METHOD_GRAD= WEIGHTED_LEAST_SQUARES
   CONV_NUM_METHOD_FLOW= ROE
   MUSCL_FLOW= YES
   SLOPE_LIMITER_FLOW= VENKATAKRISHNAN
   VENKAT_LIMITER_COEFF= 0.05
   LIMITER_ITER= 100
   TIME_DISCRE_FLOW= EULER_IMPLICIT
   CFL_NUMBER= 5
   CFL_ADAPT= YES
   CFL_ADAPT_PARAM= ( 0.5, 1.2, 1.000, 250.0 )
   LINEAR_SOLVER= FGMRES
   LINEAR_SOLVER_PREC= ILU
   ITER= 3000
   CONV_FIELD= RMS_DENSITY
   CONV_RESIDUAL_MINVAL= -6

自由流の圧力と温度は、射点の標高（``launch.altitude``）の大気の値である（上の例は海面の値）。
機体はメッシュの中に固定し、自由流の向きを迎角 :math:`\alpha` と横滑り角 :math:`\beta` だけ傾けるので、SU2 の力とモーメントの係数がそのまま機体軸の係数になる。
座標は、:math:`x` がノーズ先端から後方への距離、:math:`z` が上向き、:math:`y` が横方向である。
自由流は :math:`\alpha>0` で :math:`+z` 方向の成分を持つので、機体には :math:`+z` 向きの法線力が働く。
モーメントはノーズ先端のまわりにとり、基準長さは機体の直径 :math:`d`、基準面積は断面積 :math:`S` である。
半領域では基準面積を :math:`S/2` にするので、半モデルの力がそのまま全機の係数になる。全領域では :math:`S` である。

.. _sec-cfd-sideslip:

横滑り角とロール角速度
~~~~~~~~~~~~~~~~~~~~~~

機体座標の対気速度 :math:`(u,v,w)` から、迎角と横滑り角は :math:`\alpha=\operatorname{atan2}(w,u)`、:math:`\beta=\arcsin(v/V)` と定義する（:eq:`eq-alpha-beta`）。
SU2 には ``AOA`` に :math:`\alpha` を、``SIDESLIP_ANGLE`` に :math:`-\beta` を渡す。
SU2 の自由流の向きは CFD の座標で :math:`(\cos\alpha\cos\beta',\ \sin\beta',\ \sin\alpha\cos\beta')` で、その横滑り角 :math:`\beta'` は機体軸の :math:`\beta` と符号が逆だからである。
符号は、軸対称な表の展開（:eq:`eq-coeff-6-axisym`）との一致で確かめた（:ref:`sec-cfd-val-roll`）。
:math:`\beta=0` の設定は ``SIDESLIP_ANGLE= 0.0`` のままである。

ロール減衰微係数のケースは、機体を回転させずに、SU2 の **定常な回転座標系** で解く。
:math:`\alpha=\beta=0` で、機体がロール角速度 :math:`p` で回る流れを、次の行で与える。

.. code-block:: text

   GRID_MOVEMENT= ROTATING_FRAME
   MOTION_ORIGIN= 0.0 0.0 0.0
   ROTATION_RATE= -p 0.0 0.0

回転の中心はノーズ先端の原点で、軸は機軸（CFD の :math:`x`）である。CFD の :math:`x` は後ろ向きなので、機体軸の正のロール角速度は :math:`-x` まわりの回転になり、符号が逆になる。
角速度は無次元のロール角速度 :math:`\hat p` から固定した式

.. math::
   :label: eq-cfd-roll-rate

   p=\frac{2V}{d}\,\hat p,\qquad V=M\,a

で決める。:math:`a` は基準高度（``aero.reference_altitude``）の音速、:math:`\hat p` は ``roll_rate_hat``\ （既定 0.01）である。
Mach 数ごとに :math:`V` を決めるので、:math:`C_{lp}=\partial C_l/\partial\hat p` は飛翔計算の正規化（:eq:`eq-cl-total`）と同じ意味を持つ。

形状とメッシュ
--------------

.. _sec-cfd-surface:

壁面の二つのモード
~~~~~~~~~~~~~~~~~~

壁面の形状は ``aero.cfd.surface`` で選ぶ。

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - 値
     - 内容
   * - ``"panel_mesh"``\ （既定。``"cad"`` も同じ意味）
     - 抽出した半径分布とフィンの寸法から、形状を OpenCASCADE の立体として作り直す（:ref:`sec-cfd-rebuild`）。名前に反して、パネル法の表面メッシュは使わない
   * - ``"stl"``
     - 入力の STL をそのまま使う。重なった閉じた殻を一つの立体に統合し、レールボタンや非対称な部品もそのまま解く（:ref:`sec-cfd-stl`）。和の表面が鏡映対称と確かめられれば、1/4 領域または半領域だけをメッシュにする

.. _sec-cfd-rebuild:

形状の再構成
~~~~~~~~~~~~

``surface = "panel_mesh"`` の入力は、:doc:`geometry` で抽出した胴体の半径分布 :math:`r(x)` とフィンの寸法である。
IgnisYeet はこれを OpenCASCADE の立体として **作り直す**\ 。

1. 半径分布の折れ線を簡略化し（全長の 0.005 % 以内で頂点を間引く）、機軸のまわりに回転させて胴体の立体を作る。
2. フィンは、パネル法と同じ寸法（根元翼弦、先端翼弦、後退、スパン、厚み）の **両凸断面** を、根元と翼端の 2 つの断面から線形に補間して作る（loft）。
   根元は胴体の内側（胴体半径の最小値の半分の位置）まで延ばして、胴体と確実に交わるようにする。
3. 胴体とフィンを、ブーリアン演算の和（fuse）で一つの立体にする。
4. 遠方境界の領域（半球、球の 1/4、または箱。:ref:`sec-cfd-domain`）から、この立体を切り抜く。

この構成では、曲面が正確な曲線として残り、gmsh が自分の表面メッシャで曲率に応じてメッシュを切り直せる。
一方で、形状は回転体と台形フィンに限られ、レールボタンやポッド、非対称な突起は入らない。それらを含めて解くときは STL モードを使う。

パネル法の表面メッシュを直接使わないのは、その三角形がノーズ先端やフィンの先端が胴体と交わる付近で極端に小さくなり、
gmsh の離散曲面からの四面体分割が「頂点が線分上にある」というエラーで失敗するためである。
機体の底面は、Euler では後述の尾部フェアリング、RANS では平らな底面のままにする。

.. _sec-cfd-stl:

STL の直接利用
~~~~~~~~~~~~~~

STL は、サンプル機体のようにフィンが胴体に埋め込まれた別の閉じた立体として表されていることがあり、そのままでは体積メッシュにできない。
``surface = "stl"`` は、次の手順で STL を一つの立体にして壁面とする（:numref:`fig-cfd-stl-pipeline`）。
形状の再構成は行わないので、抽出した寸法には依存しない。

.. _fig-cfd-stl-pipeline:

.. figure:: _generated/tikz/cfd_stl_pipeline.*
   :width: 100%

   STL モードの処理の流れ。青は Rust、橙は Python（gmsh と manifold3d）で行う。底面の処理は Euler のときだけである。

1. **座標の変換**\ 。STL の点（m）を機体軸（:ref:`sec-body-basis`）で表し、CFD の座標 :math:`(x,y,z)_{\mathrm{CFD}}=(-x_b,\,y_b,\,-z_b)` に写す。
   :math:`x` はノーズ先端から後ろ向きである。STL に含まれるフィンの向き（ロール）はそのまま保ち、形状の再構成のようにフィンを :math:`+z` に合わせる回転は行わない。
   ``fin_roll_deg`` は使わない。
2. **修復**\ 。位相だけを直し、点の位置は動かさない。
   頂点を溶接し（許容は全長の :math:`10^{-6}` 倍）、縮退した三角形（頂点が重なるもの、面積が許容の 2 乗の :math:`10^{-3}` 倍未満のもの）と重複した三角形を除く。
   頂点を共有する三角形を連結成分に分け、成分ごとに辺をたどって向きをそろえ、符号付き体積が正になる（面の法線が外を向く）ように統一する。
   成分ごとに水密であることを確かめ、水密でない、3 枚以上の面が一つの辺を共有する、体積がない、のいずれかであればエラーにする。
   サンプルの STL は 5 つの成分（胴体の 14400 枚と、フィンの箱 4 つ）からなり、すべて **裏向き** であった。サンプルの STL を書き出す処理が座標軸を入れ替えて鏡映するためで、修復で向きを直す。
   修復の結果（成分ごとの三角形数、体積、向きを反転したか、除いた三角形数）は ``repair_report.json`` に残る。
3. **底面の処理**\ （Euler のとき。:ref:`sec-cfd-stl-base`）。
4. **和**\ 。Manifold :cite:`manifold` の exact なブーリアン演算（``batch_boolean`` の和）で、成分を一つの立体にする。
   成分ごとに有効な多様体であることと体積が正であることを、和については体積が正であることと種数（genus）が 0 以上であることを確かめる。
   種数が負になるのは、和が互いに離れた複数の立体になるときで、フィンが胴体に触れていないなどの入力の誤りを意味するので、エラーにする。
   結果は ``union.stl`` と ``union_report.json``\ （体積、表面積、種数、三角形数、成分の体積の和と和の体積の差である重なりの体積）に書かれる。
   和の表面は、:math:`y=0` と :math:`z=0` の平面について鏡映対称かを検査する（:ref:`sec-cfd-stl-symmetry`）。
5. **壁面の再メッシュと体積メッシュ**\ 。``stl_remesh`` による（下記）。対称と確かめた平面があれば、和をその平面で切って 1/4 領域または半領域だけをメッシュにし、そのあと鏡映して必要な領域にする。対称でなければ遠方境界は箱の全領域である。
6. **形状の忠実度**\ （:ref:`sec-cfd-fidelity`）。

``stl_remesh`` は、和の表面を体積メッシャに渡す前の処理を選ぶ。

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - 値
     - 内容
   * - ``"classify"``\ （既定）
     - gmsh の ``classifySurfaces``\ （折れ角 40 度）と ``createGeometry`` で曲面を作り、壁面の大きさ ``wall_size`` で表面を作り直す。
       STL は曲面を細長い三角形に、平らな面（フィン）を少数の大きな三角形に分けるので、等方的な三角形に置き換える。曲面の再構成に失敗したときはエラーにする。
       要素の大きさは :eq:`eq-cfd-size` と同じで、曲率の大きい部分は円周あたり 12 要素にし、最小の大きさは :math:`h_w/5` までとする（これより細かい曲率の要素は、再メッシュした面に HXT が受け付けない反転した三角形を作る）。
   * - ``"none"``
     - 和の三角形をそのままメッシュにする。サンプルの STL ではフィンの面が少数の大きな三角形で、フィンを解像できず、法線力が 3 割ほど低くなる。

体積メッシュは HXT で作る。HXT が失敗したときは、新しいプロセスで Delaunay 法をやり直す。

箱の遠方境界は、機体の中央を中心とする一辺 :math:`2\times` ``farfield`` × 全長の立方体である。
箱の 6 つの面は辺（曲線）を共有させて作る。共有しないと、HXT が「頂点が線分上にある」というエラーで失敗する。
境界には次のマーカを付ける。``wall`` と ``base`` は、メッシュの三角形の重心と法線から分ける。
重心が底面の平面 :math:`x=L` より後ろに全長の :math:`10^{-4}` 倍以上あるか、その範囲内にあって法線が :math:`+x` を向く三角形（平らな底面と、底面に揃ったフィンの後縁）が ``base`` である。
対称と確かめた形状では、箱ではなく 1/4 または半分の箱を使う（下記）。

.. _sec-cfd-stl-symmetry:

鏡映対称の検査と 1/4・半領域のメッシュ
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

和の表面を、機軸を含む二つの平面 :math:`y=0`（ピッチ面）と :math:`z=0` について別々に検査する。
検査は、位置の許容を全長の :math:`10^{-6}` 倍（サンプルで 1.8 μm）として、次の二段で行う。

1. 頂点を許容で溶接し、溶接した頂点をすべて鏡映する。鏡映した点に頂点がなければ非対称とする。
   鏡映した三角形の集合（ソートした頂点の三つ組）が元の集合と一致すれば、そこで対称と判定する。
2. 三角形の分割が鏡映の関係にないときは、表面そのものを比べる。鏡映した三角形の頂点、辺の中点、重心がすべて、元の表面から許容以内にあれば対称とする（最寄りの三角形を厳密に探す）。
   鏡映した面は、向きの違う対角線で分割した平らな四角形と同じ点の集合になる。メッシュ生成器が曲面や平面を鏡映の関係にない三角形に分割するのは普通のことで、
   サンプルの STL でも三角形の 99 % が鏡映のもの（頂点が同じ）と一致しない。
   頂点の一致は結果に記録するが、判定には使わない（三角形の分割が違えば、ブーリアン演算で生じる交線の頂点の位置が一致するとは限らない）。

平面ごとに、対称か否か、鏡映した点と表面の最大の距離（``max_mismatch``）、一致した頂点の数、三角形の分割が同一か、非対称のときの理由（最大の距離とその位置、対応する頂点のない頂点の数）を記録する。
サンプルの STL（フェアリングを含む和、8410 頂点）は、どちらの平面も対称で、最大の距離は :math:`6.1\times10^{-9}` m、頂点は全部が一致し、三角形の分割は同一ではない。
尾部フェアリングの円すいは、胴体の底面の縁が両方の平面について対称で、縁の頂点が平面上にあるとき、頂点数が 4 の倍数の環を 1/4 の領域でつないで鏡映して作るので、分割まで厳密に対称である。
縁がそうでないときは、環を角度の順につなぐだけの作り方になり、和は検査を通らない。

検査の結果と ``shape_symmetry`` から、対称性と領域が決まる。

.. list-table::
   :header-rows: 1
   :widths: 26 74

   * - ``shape_symmetry``
     - 動作
   * - ``"auto"``\ （既定）
     - 検査に通った平面だけを使う。両方なら ``"axisymmetric"``\ （1/4 領域）、:math:`y=0` だけなら ``"pitch_plane"``\ （半領域）、それ以外は ``"none"``\ （全領域）。:math:`z=0` だけが通ったときは、使える形状の対称性がないので ``"none"`` とする
   * - ``"axisymmetric"``、``"pitch_plane"``
     - 必要な平面が検査に通らなければ **エラー** にして、最大の距離と理由を示す。非対称な形状を黙って鏡映することはしない。通れば、その対称性を使う
   * - ``"none"``
     - 検査にかかわらず全領域を計算する

選んだ対称性と理由は、実行のとき標準エラー出力に表示し、``stl_fidelity.json`` と ``cfd_report.json``\ （``symmetry_check``）に平面ごとの数値とともに記録する。メッシュのハッシュには選んだ領域が含まれる。

メッシュは、対称な平面で切った和の表面から作る。三角形を平面で切って（平面上の頂点は平面に載せる）、``union_clip.stl`` に書く。
切った表面は切断面に沿って開いていて、その境界の曲線が箱の対称面を閉じる。
gmsh の ``classifySurfaces`` は、境界の曲線を折れ角が 0.1 rad（約 6 度）を超える点で分けるので、切断面が機軸と交わる点（ノーズ先端、尾部の先端）が曲線の端点になる。
対称面（``symmetry`` が :math:`y=0`、``symz`` が :math:`z=0`）は、箱の辺と、この離散曲線を並べた閉じた輪でできる平面で、1/4 領域では輪が機軸に沿って箱の隅の点とノーズ先端、尾部の先端を結ぶ。
メッシュのあとは、CAD のメッシュと同じに仕上げる。1/4 領域は :math:`z=0` で鏡映して :math:`z` について厳密に対称な半領域のメッシュにし（頂点の順序を入れ替えて向きを保ち、:math:`z=0` 上の節点は共有する）、
半領域のままにするか（``symmetry`` のマーカを持つ SU2 の半領域）、:math:`y=0` で鏡映して全領域にする。内部の対称面のマーカ（``symz``、全領域では ``symmetry`` も）は落とす。
半領域のままにするのは、``symmetry = true`` で、``betas_deg = [0]`` で、ロール減衰のケースがなく、:math:`y=0` の鏡映を含む対称性のときだけである。
:math:`z` について厳密に対称なので、``alpha_mode = "mirror"`` では :math:`-\alpha` は :math:`+\alpha` の鏡像で、解かない（:ref:`sec-cfd-run`）。

サンプルの STL（``wall_size`` 5 mm、``base_treatment = "fairing"``、HXT）の結果を示す。

.. list-table:: STL の対称性とメッシュ（サンプルの STL、``wall_size`` 5 mm）
   :header-rows: 1
   :widths: 44 18 18 20

   * - メッシュ
     - セル数
     - 全領域との比
     - 作成時間 [s]
   * - 全領域を直接（``shape_symmetry = "none"``）
     - 444163
     - 1
     - 418
   * - 1/4 領域（:math:`y\ge0`、:math:`z\ge0`）
     - 112978
     - 0.254
     - 34
   * - 半領域（1/4 領域を :math:`z=0` で鏡映、``symmetry`` あり）
     - 225956
     - 0.509
     - 34
   * - 半領域（``"pitch_plane"``、:math:`y\ge0` だけ）
     - 218692
     - 0.492
     - 62
   * - 全領域（1/4 領域を鏡映）
     - 452672
     - 1.019
     - 44

作成時間は、和と形状の忠実度の計算を含む。体積メッシュだけでは、1/4 領域が 23 s、全領域が 394 s で、17 分の 1 である。
鏡映した全領域のセル数が直接作った全領域と 2 % 違うのは、二つが別のメッシュだからである。
鏡映した全領域の壁面は、和との距離が最大 0.48 mm、体積は和より 0.15 % 小さい（直接作った全領域は 1.55 mm と 0.15 %）。
半領域のまま解くときは、壁面を :math:`y\ge0` に切った和と比べる（体積は定義しない）。

同じ 1/4 領域のメッシュから、半領域（``symmetry`` のマーカ）と、鏡映した全領域（45.3 万セル）を解いて比べた（:math:`M=0.8`、:math:`\alpha=0,4^\circ`、4 ランク）。
:math:`\alpha=4^\circ` の :math:`C_N` は、半領域が 1.1282、鏡映した全領域が 1.1212 で、差は 0.63 % である（どちらも 42 反復で残差が :math:`10^{-6}` に達した）。
鏡映した全領域は :math:`z` について厳密に対称なので、:math:`\alpha=0` の :math:`C_N` と :math:`C_Y` が 0 になる。
半領域の :math:`C_Y` は半分の機体の横力で、全機の値としては意味を持たない（表では :math:`y=0` の鏡像との平均をとるので 0 になる）。
直接作った全領域（``shape_symmetry = "none"``）は別のメッシュで、同じ設定で二度作ったメッシュ（444163 と 446325 セル）の :math:`C_N` は 1.1234（:math:`\pm4^\circ` の奇関数部分）と 1.0956 であった。
メッシュの作成は実行ごとに同一にならず（和の三角形の並びが実行ごとに違うことが原因と考えられる）、5 mm の :math:`C_N` はメッシュごとに数 % 変わる。
半領域と全領域の比較は、同じメッシュから作ったものどうしでなければ意味を持たない。

.. _sec-cfd-stl-base:

底面の処理（Euler）
~~~~~~~~~~~~~~~~~~~

非粘性の流れでは、平らな底面の後ろの流れは定まらない（:ref:`sec-cfd-fairing`）。
STL を Euler で解くときは、``base_treatment`` を **必ず** 指定する（指定がなければエラーになる）。

``"fairing"``
   底面の後ろに、尾部フェアリングの円すいを和に加える。長さは ``tail_fairing`` × 底面半径で、半径は一次関数で小さくなり、底面半径の 10 % で切って平らな面で閉じる（尖った頂点があると gmsh が曲面を作れない）。
   円すいの前面は、胴体の底面の縁の多角形と同じ形で、底面と同一平面に重なるので、和に段差や細長い三角形が残らない。
   **入力の STL を変更する** ので、警告を出す。底面の抗力は経験式で補う（:ref:`sec-cfd-post`）。

``"flat"``
   STL の形状を変えず、平らな底面を残す。ただし、この Euler の解は収束しない。
   サンプルの STL で試したところ、残差は :math:`10^{-2.5}` 付近で止まり、力の係数 :math:`C_{Fz}` は反復 400 回付近で符号が反転して振動した。
   警告を出し、収束しないケースは失敗として扱う。

どちらの場合も、``base`` マーカは力の積分から外す。RANS は後流が解の一部なので、この設定は使わず、平らな底面を残す。

.. _sec-cfd-fidelity:

形状の忠実度の報告
~~~~~~~~~~~~~~~~~~

STL モードでは、CFD の壁面が入力の形からどれだけずれたかを測って報告する。再メッシュで形が変わるので、結果の信頼性の目安になる。
比べる面は次の三つで、すべて CFD の座標で扱う。

* **入力**\ ：変換した STL（殻が重なったまま）。
* **和**\ ：修復した殻の和（フェアリングがあれば含む）。
* **CFD の壁面**\ ：SU2 のメッシュの ``wall`` と ``base`` の三角形。

それぞれについて、包含箱、三角形の数、表面積、（定義できるものは）体積を求める。入力は殻が重なるので体積を定義しない。
面どうしの距離は、サンプリングによる近似で測る。ある面の全頂点と、面積に比例して選んだ乱数の点（``wall`` の三角形数の 4 倍、2 万から 20 万の範囲。乱数は固定の種）を、
もう一方の面の最寄りの三角形へ射影し（一様格子で探索）、距離の最大値、99 パーセンタイル、二乗平均平方根を求める。比較は次の二つである。

* **CFD の壁面と和**\ ：両方向（壁面から和へ、和から壁面へ）。メッシュが元の面からずれた量を表す近似的な Hausdorff 距離である。
* **CFD の壁面と入力**\ ：``wall`` から入力への片方向。逆向きは、入力に胴体に埋まったフィンの箱の面があるので意味を持たない。
  フェアリングは入力にないので、``base`` は含めない。

距離の最大値のうち大きいほうが ``wall_size`` の半分を超えると警告を出し、``max_geometry_error``\ （既定は ``wall_size``）を超えるとエラーにする。
最大値の位置も記録するので、ずれの原因（意図した変更か、再メッシュの失敗か）を判断できる。
結果は ``<出力先>/<workdir>/mesh/stl_fidelity.json`` に保存され、``ignisyeet cfd-check 設定ファイル --mesh`` が表示する。
``cfd_report.json`` の ``fidelity`` にも、修復と和の報告とともに入る。距離はサンプリングによる近似で、細い特徴の見落としがありうる。

サンプルの STL を ``wall_size`` 2 mm、``base_treatment = "fairing"``、``shape_symmetry = "none"``\ （全領域）でメッシュにした結果を示す（``cfd-check --mesh`` の出力）。

.. list-table:: 三つの面の比較（サンプルの STL、``wall_size`` 2 mm）
   :header-rows: 1
   :widths: 26 30 22 22

   * - 面
     - 三角形の数
     - 表面積 [m\ :sup:`2`]
     - 体積 [m\ :sup:`3`]
   * - 入力の STL
     - 14448
     - 0.58497
     - 定義しない
   * - 和（種数 0）
     - 16824
     - 0.60987
     - 0.011699
   * - CFD の壁面
     - 365032（``wall`` 336015、``base`` 29017）
     - 0.60971
     - 0.011696

和の全長はフェアリングを含めて 1.77 m である。

.. list-table:: 面どうしの距離（サンプルの STL、``wall_size`` 2 mm）
   :header-rows: 1
   :widths: 28 22 22 14 14

   * - 比較
     - 最大 [m]
     - 最大の位置
     - 99 パーセンタイル [m]
     - 二乗平均平方根 [m]
   * - CFD の壁面と和（両方向）
     - :math:`6.054\times10^{-4}`
     - ノーズ先端
     - :math:`3.729\times10^{-5}`
     - :math:`9.101\times10^{-6}`
   * - CFD の壁面と入力
     - :math:`1.483\times10^{-3}`
     - :math:`x=1.501` m
     - :math:`2.286\times10^{-5}`
     - :math:`1.000\times10^{-5}`

CFD の壁面は和から 0.6 mm 以内にあり、最大はノーズ先端である。入力からの最大 1.48 mm は、フェアリングが底面とつながる位置（:math:`x=1.501` m）にあり、意図した変更である。
この値は ``wall_size`` の半分（1 mm）を超えるので警告が出る。誤差の大部分は 99 パーセンタイルの 0.02 mm 程度で、``wall_size`` よりはるかに小さい。
このメッシュは 412828 節点、1958808 個の四面体（全領域）で、SICN の最小値は 0.073、平均は 0.776、生成に 384.5 s かかった。

.. _sec-cfd-domain:

計算領域
~~~~~~~~

遠方境界の大きさは ``farfield`` × 全長（既定 20）で決め、機体の中央を中心とする。領域の形は、壁面のモードと対称性で次のように決まる。

.. list-table::
   :header-rows: 1
   :widths: 30 70

   * - 場合
     - 領域
   * - ``panel_mesh``、半領域
     - 半径 ``farfield`` × 全長の半球（:math:`y\ge0`）。:math:`y=0` の平面に ``symmetry`` の境界条件を課し、法線方向の流れがないものとする
   * - ``panel_mesh``、全領域、フィンの配置が :math:`y=0` について対称
     - 半球のメッシュを :math:`y=0` で鏡映して全球にする。``symmetry`` のマーカは付かない。全球を一度に作ると gmsh/HXT が周期の継ぎ目で失敗するためである
   * - ``panel_mesh``、全領域、フィンの配置が対称でない（3 枚フィンなど）
     - 一辺 :math:`2\times` ``farfield`` × 全長の箱。OpenCASCADE の箱は面の間で辺を共有するので、HXT がメッシュを作れる
   * - ``stl``、和の表面が :math:`y=0` について鏡映対称（検査に通った）
     - 1/4 または半分の箱を作り、``panel_mesh`` と同じに鏡映して半領域または全領域にする（:ref:`sec-cfd-stl-symmetry`）
   * - ``stl``、対称でない（または ``shape_symmetry = "none"``）
     - 一辺 :math:`2\times` ``farfield`` × 全長の箱の全領域（:ref:`sec-cfd-stl`）

半領域を使うのは、``symmetry = true``\ （既定）で、かつ次のすべてが成り立つときだけである。
形状が :math:`y=0` について対称であり（``panel_mesh`` はフィンの配置から、``stl`` は和の表面の検査で確かめる）、形状の対称性（``shape_symmetry``）に :math:`y=0` の鏡映が含まれ、``betas_deg = [0]`` で、ロール減衰のケースがない。
どれか一つでも崩れると全領域を解く。``symmetry = false`` は全領域を強制する。

``shape_symmetry`` は、鏡映像として得られるケースを解かずに省く対称性を指定する。

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - 値
     - 内容
   * - ``"axisymmetric"``
     - :math:`y=0` と :math:`z=0` の両方について鏡映対称（フィンが 4 枚で ``fin_roll_deg`` が 0 度または 45 度など）。:math:`\alpha\ge0`、:math:`\beta\ge0` だけを解く
   * - ``"pitch_plane"``
     - :math:`y=0` だけについて鏡映対称（フィンが 3 枚など）。:math:`\pm\alpha` と :math:`\beta\ge0` を解く
   * - ``"none"``
     - 対称性を使わない。:math:`\pm\alpha` と :math:`\pm\beta` を解く
   * - ``"auto"``\ （既定）
     - ``panel_mesh`` では、横滑りを解かないとき ``"axisymmetric"``\ 。横滑りを解くときは、フィンの配置から決める（両面について対称なら ``"axisymmetric"``、:math:`y=0` だけなら ``"pitch_plane"``、それ以外は ``"none"``）。``stl`` では、和の表面の鏡映検査（:ref:`sec-cfd-stl-symmetry`）に通った平面を使う（両方なら ``"axisymmetric"``、:math:`y=0` だけなら ``"pitch_plane"``、それ以外は ``"none"``）

``stl`` で ``"axisymmetric"`` か ``"pitch_plane"`` を明示したとき、和の表面が検査に通らなければエラーにする。非対称な形状を黙って鏡映して、鏡映像の係数を誤ることはない。
``"none"`` は検査にかかわらず全領域で全ケースを解く。

フィンは既定では :math:`\pm y` と :math:`\pm z` の 4 方向に付き、:math:`y=0` 面に二枚のフィンが載る。
``fin_roll_deg``\ （既定 0）でフィンの組を機軸のまわりに回転できる（45 度にするとフィンは :math:`y=0` 面に載らない）。この設定は ``panel_mesh`` だけで使う。

境界には名前（マーカ）が付いている。

.. list-table::
   :header-rows: 1
   :widths: 20 80

   * - マーカ
     - 内容
   * - ``wall``
     - 機体表面（胴体とフィン）。力を積分する面
   * - ``base``
     - 底面。Euler では尾部フェアリングの面（STL で ``"flat"`` なら平らな底面）で、力を積分しない（RANS では平らな底面で、積分する）
   * - ``farfield``
     - 遠方境界。特性量に基づく非反射の境界条件
   * - ``symmetry``
     - :math:`y=0` の平面。半領域のときだけ
   * - ``symz``
     - :math:`z=0` の平面。1/4 領域をメッシュにする途中だけにあり、鏡映して半領域にするときに落とす

.. _sec-cfd-fairing:

尾部フェアリング
~~~~~~~~~~~~~~~~

非粘性の流れでは、平らな底面の後ろの流れは問題として成り立たない。
後流が非対称な定常解を持ち、解の対称性が保てないためである。
そこで Euler では、底面の後ろに円すい形の尾部フェアリングを置く。長さは ``tail_fairing``\ （既定 6）× 底面半径で、先端は一点に閉じる。
フェアリングの面は ``base`` マーカで、力の積分から外す。
したがって底面の抗力は CFD からは得られず、部品積み上げ法の底面抗力の項で補う（:ref:`sec-cfd-post`）。
RANS では後流が解の一部なので、底面は平らなままにする。
STL モードでは ``base_treatment = "fairing"`` がこれに当たる（:ref:`sec-cfd-stl-base`）。

z 対称メッシュ
~~~~~~~~~~~~~~

機体が :math:`z` 方向にも鏡映対称（:math:`z\to-z`）であっても、非構造のメッシュは対称にならない。
半モデルのメッシュを一度に作ると、:math:`z>0` と :math:`z<0` でセルの並びが異なり、:math:`\alpha=0` でも法線力が 0 にならない（:ref:`sec-cfd-val-asym` に数値を示す）。
そこで ``z_mirror_mesh``\ （既定 ``true``）では、:math:`y\ge0` かつ :math:`z\ge0` の **1/4 領域だけ** をメッシュにし、
それを :math:`z=0` 面で鏡映して半球のメッシュにする（:numref:`fig-cfd-domain`）。
鏡映したセルは頂点の順序を入れ替えて、向きを正に保つ。:math:`z=0` の面上の節点は共有される。
こうして得た半モデルのメッシュは :math:`z` について **厳密に対称** で、:math:`-\alpha` の解は :math:`+\alpha` の解の鏡像になる（:ref:`sec-cfd-run` の ``alpha_mode``）。

.. _fig-cfd-domain:

.. figure:: _generated/tikz/cfd_domain.*
   :width: 100%

   計算領域の概念図。(a) 半球の :math:`y=0` 面での断面。機体は拡大して描き、実際の遠方境界は機体よりはるかに大きい。
   上半分（:math:`z\ge0`）だけをメッシュにし、:math:`z=0` で鏡映して下半分を作る。
   (b) 機軸の方向から見た半球。:math:`y=0` が symmetry の平面で、フィンが :math:`+y` と :math:`\pm z` に付いている。

この手法が使えるのは、形状が :math:`z\to-z` について対称なときだけである（``panel_mesh`` では、フィンが 4 枚で ``fin_roll_deg`` が 0 度または 45 度、2 枚、6 枚など。3 枚や 5 枚、roll が 10 度などは非対称）。
対称でない形状では、``z_mirror_mesh`` は自動的に無視される。
``panel_mesh`` では、フィンの配置が :math:`z\to-z` について対称なときに ``z_mirror_mesh`` が働く。``stl`` では、和の表面が :math:`z=0` と :math:`y=0` について鏡映対称と確かめられたとき（:ref:`sec-cfd-stl-symmetry`）に働く。
全領域でも、1/4 領域を :math:`z=0` と :math:`y=0` で鏡映して全領域にする。

メッシュの大きさ
~~~~~~~~~~~~~~~~

体積メッシュは四面体である（gmsh の HXT 法）。セルの大きさは、壁面までの距離 :math:`s` の関数

.. math::
   :label: eq-cfd-size

   h(s)=\min\bigl(h_{\max},\;h_w+0.2\,s\bigr),\qquad h_{\max}=\frac{R_\infty}{10}

で決める。:math:`h_w` は ``wall_size``\ （壁面の要素の大きさ、既定 2 mm）、:math:`R_\infty` は遠方境界の半径である。
壁から離れるにつれ、要素の列ごとに約 1.2 倍の割合で大きくなる。
曲率の大きい部分（ノーズ、フィンの縁）は、円周あたり 24 要素になるよう壁面の要素を細かくし、最小の大きさは :math:`h_w/8` までとする。
メッシュの生成は単一スレッドで行う。複数スレッドでは実行のたびに異なるメッシュになり、CFD の結果の再現性が損なわれるためである。

既定の 2 mm で、サンプル機体の半モデルは約 94 万セル（約 20 万節点）になる。メッシュの生成は約 14 s である。
壁面（``wall``）は 14.2 万、尾部フェアリング（``base``）は 1.5 万、対称面は 2.9 万の三角形である。

:numref:`fig-cfd-mesh-surface` に壁面のメッシュを示す。
ノーズ先端は曲率が大きいので、先端に向かって要素が細かくなる。フィンの付け根では、胴体とフィンの面が一つの立体として交わり、交線に沿って要素がつながる。

.. _fig-cfd-mesh-surface:

.. figure:: _generated/plots/cfd_mesh_surface.*
   :width: 100%

   サンプル機体の壁面のメッシュ（``wall_size`` 2 mm、半モデル）。左はノーズ先端（先端から 9 cm）、右はフィンの付け根の前縁側。

体積メッシュの様子は、対称面（:math:`y=0`）での断面で分かる（:numref:`fig-cfd-mesh-section`）。
壁面の近くでは要素が 2 mm で、壁から離れるにつれ\ :eq:`eq-cfd-size` に従って大きくなり、遠方境界（半径 30 m）では最大の大きさ 3 m に達する。

.. _fig-cfd-mesh-section:

.. figure:: _generated/plots/cfd_mesh_section.*
   :width: 100%

   対称面 :math:`y=0` での体積メッシュの断面。左から計算領域の全体、機体のまわり、フィンと尾部フェアリングの拡大。灰色の部分は機体（とフィン）の内部である。

メッシュは ``<出力先>/<workdir>/mesh/`` に保存され、形状と設定のハッシュが同じなら再利用される。

.. _sec-cfd-run:

計算ケースと実行
----------------

(M, α, β) の行列
~~~~~~~~~~~~~~~~

``machs``、``alphas_deg``、``betas_deg`` の組合せがケースの行列である（既定は 10 の Mach 数と 6 の迎角、横滑り角は 0 だけ）。
迎角は 0 を含む必要があり、横滑り角は 0 から始める。
ケースの並び順は、Mach 数が小さい順、各 Mach 数の中で横滑り角（0、正の角度の昇順、（必要なら）負の角度を絶対値の昇順）、その中で迎角を同じ順とする。
``roll_damping`` が真なら、各 Mach 数の最後にロール減衰の 2 ケースが付く（:ref:`sec-cfd-roll-cases`）。
``betas_deg`` が 0 だけでなければ、係数表は一般表になる（:ref:`sec-cfd-general`）。

``alpha_mode`` は、迎角の正負と :math:`\alpha=0` での非対称性の扱いを決める。

.. list-table::
   :header-rows: 1
   :widths: 14 86

   * - 値
     - 内容
   * - ``"mirror"``\ （既定）
     - 正の迎角を負の迎角でも解き（:math:`z` 対称メッシュのときは鏡像なので解かない）、奇関数部分と偶関数部分に分ける
   * - ``"offset"``
     - 設定した迎角だけを解き、:math:`\alpha=0` の法線力とノーズ周りのモーメントを全迎角から引く
   * - ``"single"``
     - 設定した迎角だけを解き、結果をそのまま使う

``mirror`` では、各正の迎角 :math:`\alpha` について、:math:`+\alpha` と :math:`-\alpha` の結果 :math:`f(\pm\alpha)` から

.. math::
   :label: eq-cfd-mirror

   C_N=\frac{C_N(+\alpha)-C_N(-\alpha)}{2},\qquad
   m=\frac{m(+\alpha)-m(-\alpha)}{2},\qquad
   C_A=\frac{C_A(+\alpha)+C_A(-\alpha)}{2}

を使う。:math:`m=C_N x_{cp}` はノーズ周りのモーメント（長さの次元）である。
正負の迎角で同じ値だけ現れる数値的なずれ :math:`c_0`\ （:math:`f(\pm\alpha)=\pm f_{odd}+c_0`）は、:eq:`eq-cfd-mirror` で打ち消される。
片方の迎角の解が得られなかったときは、もう一方に :math:`\alpha=0` の値をずれの推定値として使う。
:math:`\alpha=0` の解は一度だけ解き、軸力 :math:`C_A` だけを使う。その法線力とモーメント（真値は 0）は、実測された非対称性として ``cfd_report.json`` の ``asymmetry`` に記録する。
``offset`` では、:math:`\alpha=0` の値を :math:`C_N^{(0)},m^{(0)}` として

.. math::
   :label: eq-cfd-offset

   C_N=C_N(\alpha)-C_N^{(0)},\qquad m=m(\alpha)-m^{(0)}

とする。``z_mirror_mesh`` が有効なとき、:math:`-\alpha` の解は :math:`+\alpha` の解の鏡像なので、``mirror`` の組合せは :math:`+\alpha` の結果に一致する。
:math:`\alpha=0` のずれは 0 になり、負の迎角の計算は不要になる。

解く符号は ``shape_symmetry``\ （:ref:`sec-cfd-domain`）で決まる。

* ``"axisymmetric"``\ ：:math:`\alpha\ge0` と :math:`\beta\ge0` だけ。``alpha_mode = "mirror"`` で :math:`z` 対称メッシュでないときは、各正の迎角を負の迎角でも解く。
* ``"pitch_plane"``\ ：:math:`\pm\alpha` と :math:`\beta\ge0`。
* ``"none"``\ ：:math:`\pm\alpha` と :math:`\pm\beta`。

解かない符号の係数は、後処理で鏡映して補う（:ref:`sec-cfd-general`）。
:math:`n_\alpha` 個の正の迎角と :math:`n_\beta` 個の正の横滑り角を指定したとき、1 つの Mach 数の ``"none"`` のケース数は :math:`(2n_\alpha+1)(2n_\beta+1)` である。
横滑り角のケースは、そのぶん計算が増える。

.. _sec-cfd-roll-cases:

ロール減衰のケース
~~~~~~~~~~~~~~~~~~

``roll_damping = true`` にすると、各 Mach 数で :math:`\alpha=\beta=0`、:math:`\hat p=\pm` ``roll_rate_hat`` の 2 ケースを回転座標系で解く（:ref:`sec-cfd-sideslip`）。
回転があると :math:`y=0` について対称でなくなるので、全領域で解く。Euler だけで使える。
2 ケースの差からロール減衰微係数 :math:`C_{lp}` を求める（:ref:`sec-cfd-roll-post`）。差を取るので、フィンのカントや非対称による :math:`\hat p=0` のずれは打ち消される。

暖機起動と並列実行
~~~~~~~~~~~~~~~~~~

各ケースは、近いケースの収束解を初期値にして始める（暖機起動）。元のケースは次のとおりである。

* :math:`\alpha\ne0`：同じ Mach 数と横滑り角で、一つ小さい迎角（符号が同じもの、最小は :math:`\alpha=0`）。
* :math:`\alpha=0`、:math:`\beta\ne0`：同じ Mach 数の :math:`\alpha=0` で、一つ小さい横滑り角（符号が同じもの、最小は :math:`\beta=0`）。
* :math:`\alpha=\beta=0`：一つ小さい Mach 数の :math:`\alpha=\beta=0`。
* ロール減衰のケース：同じ Mach 数の回転しない :math:`\alpha=\beta=0`。

元のケースが終わる（成功でも失敗でも）まで待ち、失敗した場合は自由流から始める。
この依存関係のもとで、準備のできたケースを ``parallel_cases`` 個まで同時に実行する。
最初に :math:`\alpha=\beta=0` の列が Mach 数順に解かれ、そのあと各 Mach 数の角度の列が並列に進む。

各ケースは ``mpirun -n <ranks> --bind-to none SU2_CFD case.cfg`` で実行する（ランク数が 1 のときは ``SU2_CFD case.cfg`` を直接）。
単一ノードの実行になるので、MPI の通信は共有メモリだけにしている。
Ctrl-C や SIGTERM を受けると、子プロセスをまとめて終了し、終わったケースの結果は残す。

計算資源
~~~~~~~~

``ranks_per_case``\ （1 ケースの MPI ランク数、既定 4）と ``parallel_cases``\ （同時に動かすケース数、既定 4）の積は、使えるスレッド数以内でなければならない。
どちらも 0 にすると自動になり、ランク数は 4 とスレッド数の小さいほう、同時ケース数は ``スレッド数 / ランク数`` になる。
積が使えるスレッド数を超えるときは、まず同時ケース数を、次にランク数を減らす。

``[resources]`` の ``threads`` はスレッド数の上限（MPI ランクの総数と gmsh のスレッド数に効く）で、``memory_gb`` は同時に動かすケースのメモリの予算である。
予算に収まるまで同時ケース数を減らし（最低でも 1 ケースは動かす）、1 ケースでも予算を超えるときは警告を出す。
1 ケースのメモリは、ランク数 :math:`n_r`、セル数 :math:`N` に対して

.. math::
   :label: eq-cfd-mem

   B \approx 40\ \mathrm{MB}\times n_r+3.3\ \mathrm{kB}\times N

と見積もる（Euler。SU2 8.3.0、Roe と MUSCL、陰的な FGMRES と ILU、四面体、4 ランクで測ったピークの値から求めた）。
RANS は乱流量と勾配、大きな線形系のぶん、セルあたりの項を 1.5 倍にする。
実測では、ランクあたりの最大のメモリは 93 k セルで 266 MB、222 k セルで 332 MB、379 k セルで 444 MB、649 k セルで 629 MB、941 k セルで 819 MB、1124 k セルで 923 MB であった。
941 k セルで 4 ランクなら、:eq:`eq-cfd-mem` から約 3.3 GB で、実測の 819 MB × 4 に一致する。
``nice`` は子プロセスにも引き継がれる。

実行時間の実測を示す（Intel Core i7-12700 の 20 スレッド、メモリ 30 GB、SU2 8.3.0、Open MPI 5.0.11、gmsh 4.15.2）。
:math:`M=0.3,0.8,1.2,2.0` と :math:`\alpha=0,2,4,8^\circ` の 16 ケースを 941 k セルのメッシュで、4 ランク × 4 ケースの並列で解くと、全体で 1325 s（約 22 分）かかった。
各ケースの時間の合計は 2339 s、1 ケースは 49 s から 330 s（中央値は約 160 s）であった。

再開、収束の判定、失敗
~~~~~~~~~~~~~~~~~~~~~~

各ケースは ``<出力先>/<workdir>/m0.800_a04.00/`` のようなディレクトリに入る（負の迎角は ``a-04.00``）。
横滑り角があれば ``_b04.00``、ロール減衰のケースは ``_r+0.0100`` と ``_r-0.0100`` を名前に足す（例：``m0.800_a04.00_b04.00``）。
SU2 の設定、ログ、履歴（``history.csv``）、壁面の分布（``surface_flow.csv``）があり、終わると ``done.json``\ （入力のハッシュ、係数、収束の状態）が書かれる。
入力のハッシュは SU2 の設定（暖機起動の行を除く）とメッシュのハッシュから作る。
同じハッシュの ``done.json`` があるケースは次の実行で飛ばす。途中で止めても、あるいは設定を一部変えても、影響を受けないケースは再利用される。
最初からやり直すには、このディレクトリを削除する。

ケースの結果は、次のいずれかで扱う。

* **収束**\ ：残差の目標に達した。最後の反復の係数を使う。
* **早期終了**\ ：``min_iterations``\ （既定 100）反復を過ぎて残差が 3 桁以上下がり、直近 ``stable_window``\ （既定 60）反復の力とモーメントが落ち着いたとき（前半と後半の平均の差が力の大きさの 2 % 未満など）、ケースを止めて受け入れる（``status`` は ``accepted``）。
  ``min_iterations`` は ``stable_window`` 以上でなければならない。Euler の Roe の制限関数は ``min_iterations`` 反復で固定する（``LIMITER_ITER``）。既定値の根拠の実測は :ref:`sec-cfd-early-stop` に示す。
* **受け入れ**\ ：反復の上限か時間の上限（``timeout_minutes``、既定 0 で無制限）に達したが、残差が最大値から 3 桁（RANS は 1 桁）以上下がり、
  最後の窓で力とモーメントの変動が 1 % 未満のとき、係数を窓の平均として受け入れる。どちらの場合も「収束」には数えず、``cfd_report.json`` の ``unconverged`` に載る。
  残差は下がりきらなくても係数が平らになる状況（例えば底面の後流が振動する場合）を、結果として使うための扱いである。
* **失敗**\ ：SU2 のエラー、発散、履歴なし、あるいは収束も安定もしていない場合。``done.json`` は作らない。
  表の作成では、失敗したケースを飛ばして補間する（:ref:`sec-cfd-post`）。

:numref:`fig-cfd-convergence` に収束の履歴の例を示す。
SU2 の密度の残差（RMS の常用対数）が :math:`10^{-6}` に達するまで、:math:`M=0.8` では 90 反復ほど、:math:`M=2` では 140 から 250 反復かかった。
:math:`M=2` では途中で残差が :math:`10^{-5}` 付近にしばらくとどまるが、揚力係数は 30 反復ほどで一定の値に落ち着いている。

.. _fig-cfd-convergence:

.. figure:: _generated/plots/cfd_convergence.*
   :width: 100%

   サンプル機体の収束の履歴（Euler、一様流からの開始）。左は密度の残差、右は :math:`\alpha=4^\circ` の揚力係数。

``cfd_cases.csv`` に全ケースの状態、反復数、残差、係数（6 成分を含む）、計算時間が、``cfd_report.json`` に失敗と未収束の一覧、補間で埋めたケース、メッシュの大きさ、総計算時間、非対称性の測定値が書かれる。
``cfd_report.json`` には、さらに係数表の種類（``table_kind``）、形状の対称性と領域（``shape_symmetry``、``domain``）、横滑り角、一般表で CFD の値を使う範囲（``cfd_region``）、
ロール減衰の CFD とモデルの値と採用した側（``clp``、``clp_source``）、基準のカント（``fin_cant_ref_deg``）、底面の処理（``base_treatment``）、警告（``warnings``）、STL の形状の忠実度（``fidelity``）、
STL の鏡映対称の検査と選んだ領域（``symmetry_check``）が入る。

.. _sec-cfd-early-stop:

早期終了の実測
~~~~~~~~~~~~~~

残差が下がりきらない超音速のケースは、早期終了が計算時間を決める（:ref:`sec-cfd-run`）。
最小反復数 ``min_iterations`` と窓の長さ ``stable_window`` を、同じ 4 ケース（5 mm の半モデル）で比べた。
基準は、:math:`M=2` の二つのケースを自由流から 1000 反復まで解いた値である（早期終了なし、制限関数は ``min_iterations`` と同じ 100 反復で固定）。
:math:`\alpha=4^\circ` は残差が :math:`10^{-12}` まで下がって 177 反復で収束した。:math:`\alpha=0` は 1000 反復で :math:`10^{-7.2}` で、軸力が 5 桁で一定になっている。
:math:`M=0.8` の二つのケースは、どの設定でも反復 100 の前に :math:`10^{-6}` に達するので、設定の影響を受けない。

.. list-table:: 早期終了の設定と、基準との差（5 mm の半モデル、暖機起動）
   :header-rows: 1
   :widths: 30 14 14 14 14 14

   * - 設定
     - :math:`M=2`、:math:`\alpha=0`：反復 / 時間 [s]
     - 軸力 :math:`C_A` の基準との差
     - :math:`M=2`、:math:`\alpha=4^\circ`：反復 / 時間 [s]
     - :math:`C_N`、:math:`C_{My}`、:math:`C_A` の基準との差
     - 4 ケースの時間の合計 [s]
   * - 250 反復、窓 120
     - 251 / 174
     - +0.054 %
     - 140 / 98
     - :math:`-0.014`、:math:`-0.017`、:math:`-0.031` %
     - 370
   * - 100 反復、窓 60（既定）
     - 101 / 71
     - +0.059 %
     - 104 / 75
     - :math:`-0.015`、:math:`-0.017`、:math:`-0.033` %
     - 245

基準の値は、:math:`\alpha=4^\circ` で :math:`C_N=0.87311`、:math:`C_{My}=-9.9185`、:math:`C_A=0.089066`、:math:`\alpha=0` で :math:`C_A=0.087992` である。
:math:`\alpha=0` の :math:`C_N` と :math:`C_{My}` は、真値が 0 で、どの設定でも絶対値が :math:`10^{-4}` 以下である。
既定の 100 反復、窓 60 は、4 ケースの時間を 370 s から 245 s（34 % 減）にし、:math:`M=2` の二つでは 272 s から 146 s（46 % 減）にした。
係数は、基準から最大でも 0.06 % しか違わず、0.5 % の範囲に十分収まる。

制限関数を固定する反復を 250 から 100 に早めても、収束した係数はほとんど変わらない。:math:`M=2`、:math:`\alpha=4^\circ` を自由流から 1000 反復まで解いた基準は、
100 反復で固定したとき :math:`C_N=0.87311`\ （177 反復で :math:`10^{-12}` に収束）、250 反復で固定したとき :math:`C_N=0.87302`\ （1000 反復で :math:`10^{-8.6}`）で、差は 0.01 % である。
固定を早めるほうが、残差は早く深く下がる。

.. _sec-cfd-post:

後処理
------

SU2 の係数から機体軸の係数へ
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

機体はメッシュに固定されているので、SU2 の力の係数 :math:`C_{Fx},C_{Fz}` はそのまま機体軸の係数である。

.. math::
   :label: eq-cfd-body

   C_N=C_{Fz},\qquad C_A^{\mathrm{CFD}}=C_{Fx},\qquad m=C_N\,x_{cp}=-C_{My}\,d

SU2 はモーメントを :math:`\bm{M}=\bm{r}\times\bm{F}` で計算するので、その :math:`y` 成分は :math:`zF_x-xF_z` である。
ノーズ先端から後方 :math:`x_{cp}` に働く正の法線力は負の :math:`C_{My}` を与える。圧力中心は

.. math::
   :label: eq-cfd-xcp

   x_{cp}=\frac{m}{C_N}=-\frac{C_{My}\,d}{C_N}

である。:math:`|C_N|` が小さいときは定義しない。:math:`C_L,C_D` しか得られないときは、自由流の方向で回転して :math:`C_{Fx}=C_D\cos\alpha-C_L\sin\alpha`、:math:`C_{Fz}=C_D\sin\alpha+C_L\cos\alpha` とする。

.. _sec-cfd-body6:

6 成分の係数
~~~~~~~~~~~~

横滑りやロールを解くときは、SU2 の 6 成分の力とモーメントの係数 :math:`C_{Fx},C_{Fy},C_{Fz},C_{Mx},C_{My},C_{Mz}` を使う。
CFD の座標は機体座標を :math:`y` まわりに 180 度回したもの（:math:`\mathrm{diag}(-1,1,-1)`）で、モーメントは軸性ベクトルなので、機体軸の係数（:eq:`eq-coeff-6`）は

.. math::
   :label: eq-cfd-body6

   C_X=-C_{Fx},\quad C_Y=C_{Fy},\quad C_Z=-C_{Fz},\qquad
   C_l=-C_{Mx},\quad C_m=C_{My},\quad C_n=-C_{Mz}

となる。力は :math:`qS`、モーメントはノーズ先端まわりで :math:`qSd` で無次元化している。
:eq:`eq-cfd-body` との関係は、:math:`C_Z=-C_N`、:math:`C_m=x_{cp}C_Z/d` である（:eq:`eq-coeff-6-axisym`）。
鏡映の像は、:math:`z\to-z` で :math:`(\alpha,\beta)\to(-\alpha,\beta)` となり :math:`C_Z,C_l,C_m` の符号が変わる。
:math:`y\to-y` では :math:`(\alpha,\beta)\to(\alpha,-\beta)` となり :math:`C_Y,C_l,C_n` の符号が変わる。

軸力
~~~~

軸力 :math:`C_A` は、CFD が供給しない項を部品積み上げ法で補う。
:math:`\alpha`・Mach 数ごとの CFD の軸力 :math:`C_A^{\mathrm{CFD}}` に、:eq:`eq-cd0` の成分のうち CFD が与えない項を、迎角の補正 :math:`f(\alpha)`\ （:eq:`eq-ca-alpha`）をかけて足す。

* **Euler**\ ：CFD は圧力による抗力（造波抗力）だけを与える。底面は力の積分から外してあるので、

  .. math::
     :label: eq-cfd-ca

     C_A=\max\Bigl(C_A^{\mathrm{CFD}}+f(\alpha)\bigl(C_{D,f}+C_{D,\mathrm{base}}+C_{D,\mathrm{fin}}+C_{D,\mathrm{bt}}+C_{D,\mathrm{extra}}\bigr),\,0\Bigr)

  である。足すのは摩擦、底面、フィン縁（丸い前縁と四角い後縁）、ボートテール、追加抗力で、ノーズの圧力抗力 :math:`C_{D,\mathrm{nose}}` は **含めない**\ （CFD が与えるため）。
  燃焼中の値では、底面の項に、ノズル出口の面積を除いた底面積を使う。
* **RANS**\ ：摩擦、底面圧力、フィン、ボートテールの抗力は CFD が与えるので、追加抗力 :math:`C_{D,\mathrm{extra}}` だけを足す。
  燃焼中は、ノズル出口の面積にかかる底面圧力の寄与を引く。

亜音速の Euler では、CFD の軸力の理論値は 0（d'Alembert のパラドックス）である。実際の値は小さな負の数（本章のサンプルで :math:`-0.03` から :math:`-0.06`）になり、そのまま合計に入る。

ピッチ減衰
~~~~~~~~~~

定常な CFD はピッチ減衰を与えない。そこで :ref:`sec-damping` の和 :math:`S_0,S_1,S_2`\ （:doc:`barrowman` の\ :eq:`eq-body-sums` と同じ定義）は、
部品積み上げ法（パネル法があればその値）の **ばらつき** を残したまま、全体の傾きと重心を CFD の値に取り替える。
モデルの和を :math:`S_k^{m}` として、そのまわりのばらつき（分散）

.. math::
   :label: eq-cfd-var

   \sigma^2=\frac{S_2^m}{S_0^m}-\left(\frac{S_1^m}{S_0^m}\right)^2

を保ち、CFD の :math:`C_{N\alpha}` と小迎角の圧力中心 :math:`x_{cp0}` から

.. math::
   :label: eq-cfd-damp

   S_0=C_{N\alpha},\qquad S_1=C_{N\alpha}\,x_{cp0},\qquad S_2=C_{N\alpha}\bigl(x_{cp0}^2+\sigma^2\bigr)

とする。

係数表の作成
~~~~~~~~~~~~

解いた点は不規則な格子（解いた Mach 数と迎角）なので、それを係数表の規則的な格子に埋める。

1. 各 Mach 数の列で、:math:`C_N` とノーズ周りのモーメント :math:`m` は迎角について奇関数とし、原点 :math:`(0,0)` を通る。
   :math:`C_N,m` と CFD の軸力を、迎角について単調な 3 次補間（PCHIP :cite:`fritsch`）する。
   解いた迎角の範囲を超えるときは、端の傾きを延長する（``extrapolation = "linear"``）か、端の値で一定とする（``"clamp"``）。
2. :math:`C_{N\alpha}` と :math:`x_{cp}(\alpha\to0)` は、最小の二つの正の迎角を通る奇関数の 3 次式 :math:`c_1\alpha+c_3\alpha^3` から求める。
3. Mach 数の方向にも、各量を係数表の Mach 数の格子に PCHIP で補間する（外挿の規則は同じ）。
   途中の Mach 数の列が失敗していれば、両隣の列で補間して埋める。最小または最大の Mach 数の列の失敗は補えず、エラーになる。
4. 軸力に上の項を足し、ピッチ減衰の和を取り替える。``roll_damping`` が真のときは、:math:`C_{lp}` も CFD の値（:ref:`sec-cfd-roll-post`）にする。

係数表の入力ハッシュには ``[aero.cfd]`` の設定が含まれる。ただし CFD の実装（コード）の版は含まれない。


.. _sec-cfd-general:

一般表の作成
~~~~~~~~~~~~

係数表は、次のとき一般表（6 成分、:ref:`sec-aero-coeff6`）になる。
``surface = "stl"`` であるか、``shape_symmetry`` が ``"pitch_plane"`` か ``"none"`` と明示されているか、``betas_deg`` に 0 以外があるときである。
それ以外は軸対称表で、ロール減衰だけを解いた場合も軸対称表である。

一般表の格子は、迎角が :math:`-180^\circ` から :math:`180^\circ`、横滑り角が :math:`-90^\circ` から :math:`90^\circ` である。
どちらも非一様で、:math:`\pm30^\circ` までは 2 度刻み、その外側は 10 度刻み（迎角 61 点、横滑り角 43 点）とする。
Mach 数の刻みは ``general_mach_step``\ （既定 0.1、0 なら ``[aero]`` の ``mach_step``）で、範囲は ``[aero]`` の ``mach_min`` から ``mach_max`` である。
格子は 1 つの Mach 数あたり :math:`61\times43` 点で、Mach 数の刻みを粗くするとファイルが小さくなる。

Mach 数ごとの作り方は次のとおりである。

1. **対称性による補完**\ 。解いた点に、形状の鏡映（:ref:`sec-cfd-body6`）の像を加える。``alpha_mode = "mirror"`` では、同じ :math:`(\alpha,\beta)` に重なるすべての像を平均し、数値的な非対称を消す。
   そうでなければ、解いた点が像より優先される。``"offset"`` では、鏡映対称により :math:`(0,0)` で 0 になる成分を、先に全点から引く。
2. **CFD の補間**\ 。解いた範囲 :math:`R=[\alpha_{\min},\alpha_{\max}]\times[\beta_{\min},\beta_{\max}]` の中を、各横滑り角で迎角について PCHIP :cite:`fritsch` し、その値を横滑り角について PCHIP する（変数分離）。
3. **モデルで埋める**\ 。:math:`R` の外側は、部品積み上げ法の軸対称表を 6 成分に展開した値 :math:`T`\ （:eq:`eq-coeff-6-axisym`、全迎角 90 度まで）の変化を、:math:`R` の境界の CFD の値に合わせて足す。
   :math:`R` の内側に最も近い点を :math:`(\alpha_c,\beta_c)` として、

   .. math::
      :label: eq-cfd-fill

      C(\alpha,\beta)=F(\alpha_c,\beta_c)+T(\alpha,\beta)-T(\alpha_c,\beta_c)

   とする。境界をまたいで連続である。:math:`R` の内側では :math:`C=F` である。
   CFD の領域とモデルの領域は、表の付随ファイルではなく ``cfd_report.json`` の ``cfd_region`` と ``model_outside_cfd_region`` に記録する。
4. **軸力**\ 。CFD の :math:`C_A^{\mathrm{CFD}}=-F_{C_X}` に、:eq:`eq-cfd-ca` と同じ項を足す。迎角の補正は :math:`(\alpha_c,\beta_c)` の全迎角 :math:`\cos\alpha_t=\cos\alpha\cos\beta` から :math:`\alpha_t'=\min(\alpha_t,\pi-\alpha_t)` で決める。
   燃焼中と燃焼後の :math:`C_X` が得られる。
5. **Mach 数の方向**\ 。格子の点ごとに、解いた Mach 数の値を PCHIP で補間する（外挿の規則は ``extrapolation``）。
   最小または最大の Mach 数のケースが全部失敗していれば、補えないのでエラーにする。

ピッチとヨーの減衰の和は、零角度の傾きから別々に取り替える。
ピッチは :math:`\beta=0` で、ヨーは :math:`\alpha=0` で、奇関数の 3 次式 :math:`c_1\theta+c_3\theta^3` を最小の二つの正の角度に当てはめて傾きを求め、

.. math::
   :label: eq-cfd-slopes

   \begin{aligned}
   C_{N\alpha}^{q}&=-\frac{\partial C_Z}{\partial\alpha},&\quad
   x_{cp0}^{q}&=d\,\frac{\partial C_m/\partial\alpha}{\partial C_Z/\partial\alpha},\\
   C_{N\alpha}^{r}&=-\frac{\partial C_Y}{\partial\beta},&\quad
   x_{cp0}^{r}&=-d\,\frac{\partial C_n/\partial\beta}{\partial C_Y/\partial\beta}
   \end{aligned}

とする。:eq:`eq-cfd-damp` をピッチ（:math:`S^q`）とヨー（:math:`S^r`）にそれぞれ適用する。
横滑りを解かないときは、ヨーの和をピッチの和に等しくし、``cfd_report.json`` の ``yaw_damping_from_pitch`` が真になる。
:math:`C_{N\alpha}` と :math:`x_{cp0}` は一般表の入力ではなく、``cfd_report.json`` の ``diagnostics`` に記録する診断値である。
ピッチとヨーを別々に求めるので、機体が非対称なときは異なる値になる。

ロールのカント強制 :math:`C_{l\delta}` は、部品積み上げ法の値を使う。
一般表は、表の :math:`C_l` に含まれるカント :math:`\delta_{\mathrm{ref}}` を記録する（:eq:`eq-cl-total`）。
STL モードでは、形状（STL）にカントが含まれているので、:math:`\delta_{\mathrm{ref}}` は設定したフィンのカントである。
カントの影響は :math:`C_l` にすでに入っており、:math:`\delta=\delta_{\mathrm{ref}}` の飛翔では強制項を足さない（二重に数えない）。
``panel_mesh`` は形状にカントがないので、:math:`\delta_{\mathrm{ref}}=0` である。

.. _sec-cfd-roll-post:

ロール減衰微係数
~~~~~~~~~~~~~~~~

回転座標系の 2 ケース（:ref:`sec-cfd-roll-cases`）から、Mach 数ごとに

.. math::
   :label: eq-cfd-clp

   C_{lp}=\frac{C_l(+\hat p)-C_l(-\hat p)}{2\,\hat p}

を求める（:math:`C_l=-C_{Mx}`）。差を取るので、カントや非対称による零回転のずれは消える。
次の場合、その Mach 数の値は使わず、部品積み上げ法の :math:`C_{lp}` に替えて、理由を警告と ``cfd_report.json`` の ``clp`` に記録する。

* 2 ケースのどちらかが失敗したか、値が有限でない。
* フィンがあるのに :math:`C_{lp}` が正である（ロール減衰は負でなければならない）。
* フィンがあるのに :math:`|C_{lp}|<10^{-6}` である。

表の値は、部品積み上げ法の :math:`C_{lp}(M)` に、使える Mach 数での CFD / 部品積み上げ法の比を PCHIP で補間して（範囲外は端の値で一定）かけた値である。
使える Mach 数がなければ部品積み上げ法の値になる。``clp_source`` は ``"cfd"``\ （すべての Mach 数が CFD）、``"barrowman"``\ （解かない、またはすべて不採用）、``"mixed"`` のいずれかである。
ロール減衰は Euler の解であり、粘性の寄与（フィンの摩擦）は含まない。

.. _sec-cfd-flow:

サンプル機体の流れ場
--------------------

係数表の値の裏にある流れ場を、サンプル機体の :math:`\alpha=4^\circ` で示す。
条件は :math:`M_\infty=0.8` と :math:`2.0`、メッシュは既定の 2 mm（94 万セル）、モデルは Euler である。
図のデータは SU2 の解（``restart.dat``）から ``doc/scripts/extract_cfd_fields.py`` で取り出し、``doc/data/cfd/fields/`` に保存した。

:numref:`fig-cfd-mach-field` は対称面での Mach 数の分布である。色は一様流の Mach 数 :math:`M_\infty` を中心とし、青が一様流より遅い領域、橙が速い領域を表す。

* :math:`M_\infty=0.8`\ （上）：ノーズ先端によどみ点（青）があり、ノーズの肩で流れが加速する（橙）。
  フィンの前縁の手前と、尾部フェアリングに沿った流れは減速している。
* :math:`M_\infty=2.0`\ （下）：ノーズ先端から斜めに広がる青い帯が、ノーズの衝撃波である。迎角があるので、風上側（:math:`z<0`）の衝撃波のほうが強い。
  ノーズと円筒の境目では膨張して加速し、フィンの前縁からも衝撃波が出ている。

.. _fig-cfd-mach-field:

.. figure:: _generated/plots/cfd_mach_field.*
   :width: 100%

   対称面 :math:`y=0` での Mach 数の分布（:math:`\alpha=4^\circ`、上は :math:`M_\infty=0.8`、下は :math:`M_\infty=2.0`）。
   流れは左から右へ向かう。色の中心は一様流の Mach 数で、範囲を超える値は端の色で示す。灰色は機体の内部である。

:numref:`fig-cfd-cp-surface` は壁面の圧力係数 :math:`C_p` の分布で、半モデルの解を :math:`y=0` 面で鏡映して全機を描いた。
機体の下側（風上側）から見ている。

* ノーズでは風上側の :math:`C_p` が風下側より高く、この差が法線力を生む。
  先端から 0.02〜0.25 m の平均は、:math:`M_\infty=0.8` で風上側 :math:`-0.02`、風下側 :math:`-0.09`、:math:`M_\infty=2.0` で風上側 0.10、風下側 0.03 である。
* フィンの前縁はよどみ点に近く、正圧（橙）になる。
* :math:`M_\infty=0.8` では、尾部フェアリングの付け根（底面の縁）で流れが曲がって加速し、環状の負圧ができる（最小で :math:`C_p\approx-1.0`）。

.. _fig-cfd-cp-surface:

.. figure:: _generated/plots/cfd_cp_surface.*
   :width: 100%

   壁面の圧力係数（:math:`\alpha=4^\circ`、上は :math:`M_\infty=0.8`、下は :math:`M_\infty=2.0`）。左はノーズ、右はフィンと尾部フェアリング。
   色の範囲は図ごとに異なる。

.. _sec-cfd-val:

検証
----

本節は、CFD の実装を確かめる計算（検証）と、手法どうしの比較（相互比較）を示す。実際の機体の測定との比較（妥当性確認）は含まない。
数値は、特に断らない限り ``doc/data/cfd/`` に保存した実測値である。ドキュメントのビルドでは SU2 を実行せず、このデータから表と図を作る。

円すいの表面圧力（Taylor–Maccoll）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

半頂角 10 度の円すいと円筒の組合せを :math:`M=2`、:math:`\alpha=0` で解き、円すい表面の圧力係数（円すい長さの 0.4 から 0.95）を
Taylor–Maccoll の解 :cite:`taylormaccoll`\ （:doc:`panel` の局所傾斜法と同じ、0.10448）と比べる（Euler、Roe、半モデル）。

.. list-table:: 円すいの表面圧力係数 :math:`C_p`\ （:math:`M=2`）。メッシュを細かくする
   :header-rows: 1
   :widths: 18 18 18 18 14 14

   * - ``wall_size`` [mm]
     - セル数
     - 平均 :math:`C_p`
     - 標準偏差
     - 理論値
     - 誤差 [%]
   * - 10
     - 38775
     - 0.09274
     - 0.00598
     - 0.10448
     - :math:`-11.23`
   * - 5
     - 74350
     - 0.09886
     - 0.00364
     - 0.10448
     - :math:`-5.38`
   * - 2.5
     - 189199
     - 0.10259
     - 0.00125
     - 0.10448
     - :math:`-1.80`

メッシュを細かくすると平均値が理論値に近づき、ばらつきも小さくなる（誤差は 11.2 % から 5.4 %、1.8 % へ減る）。
粗いメッシュでは、数値的な拡散のために、円すい表面の圧力が理論値より低く出ると考えられる。

.. _sec-cfd-val-asym:

α = 0 の非対称性
~~~~~~~~~~~~~~~~

機体は対称なので、:math:`\alpha=0` の法線力 :math:`C_N` は 0 のはずである。
サンプル機体（Euler、``wall_size`` 10 mm）の半モデルで :math:`\alpha=0,\pm4^\circ` を解き、:math:`\alpha=0` の値と、
:math:`\pm4^\circ` の和 :math:`C_N^{even}=(C_N(+4^\circ)+C_N(-4^\circ))/2`\ （:math:`\alpha=0` のずれと同じもの）、差 :math:`C_N^{odd}=(C_N(+4^\circ)-C_N(-4^\circ))/2`\ （:eq:`eq-cfd-mirror`）を調べた。

.. _tbl-cfd-asym:

.. list-table:: :math:`\alpha=0` の法線力係数 :math:`C_N^{(0)}`\ （真値は 0）と、:math:`\alpha=4^\circ` の奇関数部分 :math:`C_N^{odd}`
   :header-rows: 1
   :widths: 30 10 15 15 15 15

   * - 条件
     - :math:`M`
     - :math:`C_N^{(0)}`
     - :math:`C_N^{even}`\ （:math:`4^\circ`）
     - :math:`C_N^{odd}`\ （:math:`4^\circ`）
     - 備考
   * - 半モデル（87958 セル）
     - 0.3
     - :math:`-0.04684`
     - :math:`-0.05537`
     - 1.1788
     -
   * -
     - 0.8
     - :math:`-0.06276`
     - :math:`-0.06877`
     - 1.1637
     -
   * -
     - 2
     - :math:`-0.01724`
     - :math:`-0.01881`
     - 0.8021
     -
   * - フィンを 45 度回転
     - 0.3
     - :math:`-0.05764`
     - :math:`-0.05349`
     - 1.2047
     - フィンが :math:`y=0` 面にない
   * -
     - 0.8
     - :math:`-0.04452`
     - :math:`-0.04232`
     - 1.1628
     -
   * -
     - 2
     - :math:`-0.06309`
     - :math:`-0.05987`
     - 0.8019
     -
   * - JST
     - 0.3
     - 0.09096
     - 0.08504
     - 1.0398
     -
   * -
     - 0.8
     - :math:`-0.03309`
     - :math:`-0.03955`
     - 1.0655
     -
   * -
     - 2
     - :math:`-0.01267`
     - :math:`-0.01383`
     - 0.7336
     -
   * - 残差の目標 :math:`10^{-9}`\ （6000 反復）
     - 0.3
     - :math:`-0.04652`
     - :math:`-0.05514`
     - 1.1789
     -
   * -
     - 0.8
     - :math:`-0.06276`
     - :math:`-0.06877`
     - 1.1638
     -
   * -
     - 2
     - :math:`-0.01723`
     - :math:`-0.01891`
     - 0.8021
     -
   * - z 対称メッシュ（92668 セル）
     - 0.3
     - 0.00000
     - 0.00012
     - 1.2143
     - 1/4 領域を鏡映
   * -
     - 0.8
     - 0.00000
     - 0.00000
     - 1.1761
     -
   * -
     - 2
     - 0.00000
     - 0.00001
     - 0.7962
     -

:numref:`fig-cfd-asym` に :math:`\alpha=0` の値を示す。

.. _fig-cfd-asym:

.. figure:: _generated/plots/cfd_asymmetry.*
   :width: 100%

   :math:`\alpha=0` の法線力係数（真値は 0）。条件ごとに :math:`M=0.3,0.8,2` を並べた。
   z 対称メッシュでは 0 になる（:math:`|C_N|<10^{-3}`）。

半モデルの非構造メッシュでは、真値が 0 の :math:`C_N` が :math:`-0.017` から :math:`-0.063`\ （JST では :math:`+0.091` まで）の大きさで残る。
この大きさは Mach 数、スキーム、メッシュによって変わる。
まず、フィンを回して :math:`y=0` 面から外してもずれは消えず、残差の目標を 10\ :sup:`-9` に下げても変わらない（:math:`-0.04684` に対して :math:`-0.04652`）ので、フィンの配置や反復の不足が原因ではない。
スキームを変えると値が変わる（JST で符号まで変わる）ので、原因は数値的なものである。
また、:math:`\alpha=4^\circ` の偶関数部分 :math:`C_N^{even}` は :math:`\alpha=0` のずれとほぼ同じ大きさで、迎角に関わらず一定のずれが載っていることを示す。
これは、:eq:`eq-cfd-mirror` の組合せで打ち消される（奇関数部分 :math:`C_N^{odd}` に影響しない）。
一方、z 対称メッシュでは :math:`\alpha=0` のずれが 0 になり、組合せの手間も不要になる。
ただし奇関数部分は、非対称メッシュの結果と最大約 3 %（:math:`M=0.3`：1.1788 に対し 1.2143）異なる。これはメッシュそのものの違いも含む。

メッシュの収束
~~~~~~~~~~~~~~

``wall_size`` を 10 mm から 1.8 mm まで変えて（セル数 92668 から 1124342）、サンプル機体の :math:`\alpha=4^\circ` の結果を比べた（半モデル、z 対称メッシュ）。

.. _tbl-cfd-mesh:

.. list-table:: メッシュの収束（サンプル機体、:math:`\alpha=4^\circ`）。誤差は外挿値 :math:`C_N^\infty` に対する値
   :header-rows: 1
   :widths: 14 16 14 14 14 14 14

   * - ``wall_size`` [mm]
     - セル数
     - :math:`C_N`\ （:math:`M=0.8`）
     - 誤差 [%]
     - :math:`C_N`\ （:math:`M=2`）
     - 誤差 [%]
     - :math:`x_{cp}` [m]（:math:`M=2`）
   * - 10
     - 92668
     - 1.1761
     - :math:`-3.12`
     - 0.7962
     - :math:`-15.25`
     - 1.0883
   * - 5
     - 222158
     - 1.1939
     - :math:`-1.65`
     - 0.8730
     - :math:`-7.07`
     - 1.1359
   * - 3.5
     - 379374
     - 1.2259
     - 0.98
     - 0.8996
     - :math:`-4.23`
     - 1.1472
   * - 2.5
     - 649110
     - 1.2145
     - 0.05
     - 0.9123
     - :math:`-2.88`
     - 1.1525
   * - 2
     - 941082
     - 1.2098
     - :math:`-0.34`
     - 0.9188
     - :math:`-2.19`
     - 1.1536
   * - 1.8
     - 1124342
     - 1.2021
     - :math:`-0.98`
     - 0.9259
     - :math:`-1.43`
     - 1.1551

代表的な格子の幅を :math:`h=N^{-1/3}`\ （:math:`N` はセル数）として、一般化した Richardson 外挿 :cite:`roache` の形

.. math::
   :label: eq-cfd-richardson

   f(h)=f_\infty+C\,h^{p}

に当てはめて、外挿値 :math:`f_\infty` と収束の次数 :math:`p` を求める（:numref:`fig-cfd-conv`）。

.. _fig-cfd-conv:

.. figure:: _generated/plots/cfd_mesh_convergence.*
   :width: 80%

   :math:`C_N`\ （:math:`\alpha=4^\circ`）の格子幅 :math:`h=N_{cell}^{-1/3}` による変化。
   :math:`M=2` は\ :eq:`eq-cfd-richardson` の曲線と外挿値（白抜き）、:math:`M=0.8` は外挿値 1.214（点線）を示す。

* :math:`M=2`\ ：:math:`C_N` は単調に増えて外挿値に近づく。当てはめは :math:`p=2.65`、:math:`C_N^\infty=0.9394` である。
  既定の 2 mm では外挿値より 2.2 % 低い。細かい二つのメッシュ（941 k と 1124 k セル）の差から、格子収束指数（GCI、安全率 1.25）

  .. math::
     :label: eq-cfd-gci

     \mathrm{GCI}=\frac{1.25\,|\varepsilon|}{r^{p}-1},\qquad \varepsilon=\frac{f_2-f_1}{f_1},\quad r=\left(\frac{N_1}{N_2}\right)^{1/3}

  は 5.6 % である。:math:`f_1,f_2` は細かいほうと粗いほうの解で、収束値の不確かさを保守的に見積もる値である。
* :math:`M=0.8`\ ：:math:`C_N` は単調に収束せず、5 mm 以下では約 :math:`\pm1` % の範囲で振動する（1.194、1.226、1.215、1.210、1.202）。
  外挿値は 1.214 だが、単調でないので GCI は定義できない。
  非構造メッシュの個々のセルの並びで、わずかな差が出るためである。
* :math:`M=2` の :math:`x_{cp}` は 2 mm と 1.8 mm の差が 0.12 % である。:math:`M=0.8` の :math:`x_{cp}` は 1.2163 m と 1.2125 m で、差は 0.3 % である。
* 圧力による軸力 :math:`C_A^{\mathrm{CFD}}`\ （:math:`M=2`）は 10 mm の 0.0726 から 1.8 mm の 0.1065 まで増え、2 mm と 1.8 mm の差は約 1 % である。法線力よりゆっくり収束する。

したがって、既定の ``wall_size`` = 2 mm は、:math:`C_N` が収束値から 2 % 程度の範囲に入る設定であり、5 mm では :math:`M=2` で 7 % ほど低くなる。
一方で 5 mm は、セル数が 222 k と 941 k で 4.2 倍、:math:`M=2` の 1 ケースの時間が 40.1 s と 184.8 s で 4.6 倍と、コストは 4 から 5 分の 1 である。

遠方境界の大きさ
~~~~~~~~~~~~~~~~

:math:`M=0.8`、:math:`\alpha=4^\circ`、``wall_size`` 2 mm（z 対称メッシュ）で、遠方境界の半径を 10、20、40 倍の全長に変えた。

.. list-table:: 遠方境界の半径と結果
   :header-rows: 1
   :widths: 16 20 16 16 16 16

   * - 半径 [全長]
     - セル数
     - :math:`C_N`
     - :math:`x_{cp}` [m]
     - :math:`C_A^{\mathrm{CFD}}`
     - :math:`C_N` の差（20 倍に対して）[%]
   * - 10
     - 940588
     - 1.2071
     - 1.2128
     - :math:`-0.03955`
     - :math:`-0.22`
   * - 20
     - 941082
     - 1.2098
     - 1.2163
     - :math:`-0.03829`
     - 0.00
   * - 40
     - 943850
     - 1.2083
     - 1.2187
     - :math:`-0.03719`
     - :math:`-0.12`

:math:`C_N` は 0.25 % 以内で一致し、遠方境界の位置の影響は小さい。:math:`x_{cp}` は 20 倍と 40 倍で約 0.2 % 変わる。
壁面のメッシュの大きさを保ったまま遠方境界を広げても、セル数はほとんど増えない（10 倍の半径から 40 倍で 0.3 % 増）。

サンプル機体: 部品積み上げ法とパネル法との比較
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

サンプル機体を、Euler の CFD（半モデル、``wall_size`` 2 mm、94 万セル、z 対称メッシュ、``alpha_mode = "mirror"``）で、:math:`M=0.3,0.8,1.2,2.0` と :math:`\alpha=0,2,4,8^\circ` について解いた。
3 ケース（:math:`M=1.2` の :math:`\alpha=0`、:math:`M=1.2` の :math:`\alpha=8^\circ`、:math:`M=2` の :math:`\alpha=8^\circ`）は、早期終了の最小反復数 250（``min_iterations = 250``、``stable_window = 120``）の時点で係数が落ち着いたので、早期終了して受け入れた（残差は :math:`10^{-4.3}` から :math:`10^{-5.6}`）。
残りの 13 ケースは残差の目標 :math:`10^{-6}` に達した。

結果は、3 手法を並べて :doc:`aerodynamics` の :numref:`tbl-cfd-compare` と :numref:`fig-cfd-compare` に示した。
この結果から次のことが言える。

* **亜音速**\ （:math:`M=0.3,0.8`）：CFD の :math:`C_{N\alpha}` はパネル法と 2 % 以内で一致する（15.17 対 14.84、16.40 対 16.10）。独立な二つの非粘性の方法が一致したことになる。
  Barrowman 法は CFD より約 23 から 25 % 小さい（CFD は Barrowman 法の 1.30 倍と 1.34 倍）。パネル法の章で見た Barrowman 法との差（:doc:`panel`）を、CFD も同じ方向に示した。
* **遷音速**\ （:math:`M=1.2`）：CFD の :math:`C_{N\alpha}` は Barrowman 法より 32 % 大きく、パネル法（遷音速の補間値）より 8 % 小さい。
  :math:`C_N(4^\circ)` では、CFD の 1.315 に対して Barrowman 法は 1.081、パネル法は 1.054 と、18 から 20 % 小さい。
* **超音速**\ （:math:`M=2`）：CFD の :math:`C_{N\alpha}` は 13.04 で、Barrowman 法（11.19）より 17 % 大きく、パネル法（9.95）より 31 % 大きい。
  パネル法の局所傾斜法は、フィンと胴体の干渉を含まないために低めに出ると考えられる。
* **圧力中心**\ ：CFD は Barrowman 法より 0.015 から 0.083 m 後ろ（機体の後方）にある。パネル法とは 0.03 から 0.05 m の差である。
* **軸力**\ ：CFD に部品積み上げ法の摩擦等の項を足した :math:`C_A` は、:math:`M=2` で Barrowman 法とのずれが 3 % 以内（0.573 対 0.558）、
  亜音速では Barrowman 法より 8 から 16 % 低い。亜音速の差は、先に述べた CFD の軸力が負になる分を含む。

これらは、同じ非粘性の仮定の上での比較であり、風洞試験などの実測との比較ではない。どの値が実機に近いかは、この比較だけでは決まらない。

.. _sec-cfd-val-roll:

ロール減衰と符号の確認
~~~~~~~~~~~~~~~~~~~~~~

回転座標系によるロール減衰と、横滑りを含む 6 成分の符号を、コードの確認として調べた。
実測との比較ではなく、座標の取り方、符号、正規化が意図したとおりかどうかを確かめるものである。
:math:`M=0.8` で、壁面 5 mm の半モデルのメッシュを :math:`y=0` で鏡映した粗い全領域のメッシュ（約 44 万セル）を使った。

**ロール減衰**\ 。サンプル機体と、フィンのない回転体について、次の結果を得た。

.. list-table:: 回転座標系のロールモーメント（:math:`M=0.8`、粗い全領域のメッシュ）
   :header-rows: 1
   :widths: 40 30 30

   * - 条件
     - :math:`C_{Mx}`
     - :math:`C_l=-C_{Mx}`
   * - 回転なし（サンプル機体）
     - ほぼ 0
     - ほぼ 0
   * - サンプル機体、:math:`\hat p=+0.01`
     - :math:`+0.2162`
     - :math:`-0.2162`
   * - サンプル機体、:math:`\hat p=-0.01`
     - :math:`-0.2162`
     - :math:`+0.2162`
   * - サンプル機体、:math:`\hat p=+0.005`
     - :math:`+0.1095`
     - :math:`-0.1095`

* 回転がなければロールモーメントは 0 である。フィンのない回転体では :math:`C_{lp}\approx-0.001` で、ほぼ 0 になる（非粘性の流れで、回転体に回転を妨げるモーメントはない）。
* サンプル機体は :math:`\hat p=\pm0.01` の差から :eq:`eq-cfd-clp` で :math:`C_{lp}=-21.6` となる。符号は負で、ロールを減衰させる向きである。
* :math:`\hat p=0.005` と 0.01 の :math:`C_{Mx}` は比例し、直線からのずれは 1.3 % である。したがって :math:`C_{lp}` は :math:`\hat p` によらない。
  これは :eq:`eq-cfd-roll-rate` の無次元化（:math:`p=2V\hat p/d`）が正しいことの確認にもなる。
* :math:`\hat p>0` の :math:`C_{Mx}` が正で、機体軸の :math:`C_l=-C_{Mx}` は負になる。

メッシュを変えたときの :math:`C_{lp}` の変化は、まだ確かめていない。

**符号**\ 。軸対称な表を 6 成分に展開した値（:eq:`eq-coeff-6-axisym`）との一致を調べた。

* :math:`\alpha=4^\circ`：:math:`C_{Fz}=+1.193`、:math:`C_{My}=-14.61` から :math:`C_Z=-1.193`、:math:`C_m=-14.6` となる。
  展開した :math:`C_m=x_{cp}C_Z/d` と一致する。
* SU2 の ``SIDESLIP_ANGLE`` を :math:`+4^\circ`\ （:math:`\beta=-4^\circ`）にした場合：:math:`C_{Fy}=+1.204`、:math:`C_{Mz}=+14.78` から :math:`C_Y=+1.204`、:math:`C_n=-14.78` となる。
  展開した値（:math:`C_n=-14.7`）と一致する。

これらから、写像 :eq:`eq-cfd-body6` と ``SIDESLIP_ANGLE`` が :math:`-\beta` であることを確かめた。
STL モードの形状での比較は、この確認に含めていない。

計算時間
~~~~~~~~

上の 16 ケースは、4 ランク × 4 ケースの並列で全体の時間が 1325 s、ピークのメモリはランクあたり 819 MB であった。
メッシュの生成は 14 s である。メッシュの収束の計算（:math:`\alpha=4^\circ` の 1 ケース、4 ランク × 4 ケースの並列）では、
2 mm（94 万セル）の計算時間が :math:`M=0.8` で 55.9 s、:math:`M=2` で 184.8 s であった。
1.8 mm では、それぞれ 65.8 s、141.6 s である。粗い 10 mm（9.3 万セル）では 4.6 s と 17.6 s である。
超音速の計算は収束までの反復が増え、:math:`M=2` は :math:`M=0.8` の 3 倍ほどかかる。

.. _sec-cfd-multigrid:

マルチグリッドの実測
~~~~~~~~~~~~~~~~~~~~

``multigrid``\ （SU2 の ``MGLEVEL``）で凝集型マルチグリッドが使えるかを、サンプル機体の壁面 5 mm の半モデル（22.2 万セル、4.8 万節点、境界の三角形 4.0 万）で測った。
:math:`M=0.8,2` と :math:`\alpha=0,4^\circ` の 4 ケースを、4 ランク、暖機起動の順で解いた（``min_iterations`` 250、``stable_window`` 120）。

.. list-table:: ``MGLEVEL`` と反復数、計算時間（4 ケース）
   :header-rows: 1
   :widths: 30 17 17 17 19

   * - 設定
     - :math:`M=0.8`、:math:`\alpha=0`
     - :math:`M=0.8`、:math:`\alpha=4^\circ`
     - :math:`M=2`、:math:`\alpha=0`
     - :math:`M=2`、:math:`\alpha=4^\circ`
   * - ``multigrid = 0``\ （反復数 / 時間 [s]）
     - 93 / 67
     - 43 / 31
     - 251 / 174
     - 140 / 98
   * - ``multigrid = 2``、W サイクル
     - 93 / 70
     - 43 / 32
     - 251 / 180
     - 140 / 87

二つの設定は、反復数も残差の履歴（``history.csv``）も完全に一致した。時間の差は、同じ計算機の他の負荷による揺らぎである。
SU2 のログを見ると、``MGLEVEL = 2`` でも ``Setting the multigrid structure.`` のあとに粗い格子の表（``MG Level | CVs | Aggl. Rate``）が出ず、粗い格子の解法器も作られない。
SU2 は、要求した段数を作れないときも、その理由を出力しない。
同じ設定を球の周りの四面体メッシュで試すと、``MGLEVEL = 2`` で 2 段の粗い格子ができた（1 段あたり 1/4 から 1/6 に凝集する）。
壁面の要素を細かくした球の周りのメッシュ（2.95 万節点、壁面の三角形 1.5 万）では 1 段（1/2.6）しかできず、サンプルの CAD のメッシュ（4.8 万節点）と STL のメッシュ（9.1 万節点と、20 mm の 2.7 万節点）では 1 段も作られない。
境界の節点が多い四面体メッシュは、境界の節点が同じ境界の節点とだけ凝集するために、凝集が進まないと考えられる。
粗い格子ができるメッシュでは、回転座標系（``GRID_MOVEMENT = ROTATING_FRAME``）と陰的時間積分に ``MGLEVEL`` を組み合わせた設定を SU2 が受け付けて計算が走る（球のメッシュで 8 反復を確かめた）。

したがって、``multigrid`` の既定は Euler と RANS のどちらも 0 で、設定しても効果がない。
粗い格子が作られないとき、ランナーは「要求した段数のうち作られた段数」を標準エラー出力に一度だけ表示する。

.. _sec-cfd-rans:

RANS のケース（動作の確認）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

``model = "rans"`` で動かす経路は用意してあるが、**結果は検証できていない**\ 。実行できることの確認として、一つのケースの結果を示す。

サンプル機体を :math:`M=0.8`、:math:`\alpha=0,4^\circ` で、``wall_size`` 5 mm（19.7 万セル、半モデル、:math:`\alpha=0` のずれを引く ``offset`` 形式）、4 ランクで解いた。
計算時間は 69 s と 85 s である。どちらも 300 反復の上限で打ち切られ、受け入れた（残差の最終値は :math:`10^{-5.4}` と :math:`10^{-3.5}`）。
得られた :math:`C_N(4^\circ)` は 1.506、圧力中心は 0.945 m で、Euler の結果（1.218 と 1.220 m）や、Barrowman 法（0.949 と 1.137 m）、パネル法（1.216 と 1.174 m）のどれとも合わない。
圧力中心が 0.27 m も前に出ており、物理的な結果とは考えにくい。

原因は壁面のメッシュにある。gmsh 4.15 では OpenCASCADE の曲面に対して 3 次元の境界層（プリズム層）を作れないので、壁面のセルは四面体のままである。
そのため壁面の最初のセルの高さを :math:`y^+\approx1` にできず、実際の :math:`y^+` は広くばらつく。
壁面の :math:`y^+` の分布を 279 点で調べると、最小 0、中央値 30、90 パーセンタイル 475、最大 1299、平均 117 であった。
壁関数が想定する範囲（約 30 から 300）を大きく外れる節点が多い。
さらに、2 次精度の再構成は四面体の壁面メッシュで発散したので、1 次精度の Roe を使っている。
1 次精度では数値粘性が大きく、境界層やはく離の流れの再現精度が低い。

以上から、現状の RANS は、解の経路が動くことの確認にとどまり、係数表に使える精度ではない。

制約
----

本実装の CFD モードには次の制約がある（:doc:`limitations` にも挙げた）。

* **境界層のプリズム層がない**\ ：四面体だけのメッシュなので、壁面の摩擦を解像できない。RANS は信頼できず、既定の Euler に部品積み上げ法の摩擦を足す方法を使う。
* **メッシュ収束が完全でない**\ ：既定の 2 mm で :math:`C_N` が収束値から約 2 % の範囲にある。:math:`M=0.8` は単調に収束せず、約 1 % の振動を残す。
* **底面と尾部の扱い**\ ：Euler では尾部フェアリングで底面の後流を避けるため、底面抗力は経験式による。亜音速の軸力は小さな負の値になる。
  STL を Euler で解くには ``base_treatment`` が必要である。``"fairing"`` は入力の STL を変更する。``"flat"`` は形状を保つが、Euler の解は収束しなかった（サンプルの STL で確認）。
* **ピッチとヨーの減衰は与えない**\ ：定常計算なので、:math:`C_{mq}` と :math:`C_{nr}` は部品積み上げ法（またはパネル法）の和のばらつきに、CFD の傾きと重心を合わせた近似である。
  ロール減衰 :math:`C_{lp}` だけは、回転座標系の定常解から得る。ロールとピッチ・ヨーの交差する微係数は扱わない。
* **ロール減衰の確認が限られる**\ ：回転座標系の Euler の解で、粘性の寄与は含まない。メッシュを変えたときの値の変化は確かめていない。
* **STL の対称性の検査は和の表面の幾何だけ**\ ：``stl`` の鏡映対称は、和の表面を全長の :math:`10^{-6}` 倍（サンプルで 1.8 μm）の許容で検査する（:ref:`sec-cfd-stl-symmetry`）。
  STL の三角形分割が曲面を鏡映の関係にない平面で近似していれば、人の目に対称でも検査に通らず、全領域を解く。許容は変えられない。
  鏡映像として補う係数は、検査に通った平面についてだけ使う。
* **一般表は CFD の領域の外でモデルの値**\ ：解いた迎角と横滑り角の範囲の外側は、部品積み上げ法の展開値を境界の CFD の値に合わせて延ばす。軸対称な形状の展開なので、非対称な形状の外側では当てにならない。
* **形状の忠実度の値は近似**\ ：サンプリングによる距離の推定で、細い特徴（ごく薄いフィン縁など）のずれを見落とすことがある。
* **係数表のハッシュ**\ ：``[aero.cfd]`` の設定とメッシュは含むが、CFD の実装の版は含まない。コードを更新したときは、``workdir`` を削除して計算し直す。
* **プラットフォーム**\ ：SU2、gmsh、Open MPI は Linux 向けの conda-forge パッケージで、manifold3d は pip のパッケージ（3.5.4）で動作を確認した。他の環境では確かめていない。
* **計算コスト**\ ：既定の行列（10 Mach × 6 迎角）を 2 mm で解くと、大量のメモリと時間を使う。メモリは\ :eq:`eq-cfd-mem` で見積もれる。
  粗い 5 mm は 4 から 5 分の 1 の時間で済むが、:math:`M=2` で :math:`C_N` が最大 7 % ほど低い。
  全領域は半領域の 2 倍のセルになる（サンプルの STL の 2 mm で約 200 万セル）。横滑り角や負の迎角を解くと、ケース数がそのぶん増える。
  STL が対称なら半領域または 1/4 領域のメッシュで済む。非対称な STL、横滑り角やロール減衰のケースがあるときは全領域である。
