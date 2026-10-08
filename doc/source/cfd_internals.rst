CFD の内部と数値計算法
======================

本章は、:doc:`cfd` で使う CFD が内部で何をしているかを、実装を確かめたい読者のために説明する。
外部プログラムの役割、支配方程式と SU2 の設定、形状とメッシュの作り方、ケースの実行と収束の扱い、結果から係数表を作る後処理の順に述べる。
使い方と結果の読み方は :doc:`cfd` にあり、実装を確かめた計算（検証）と手法どうしの比較は :ref:`sec-vaero-compare` にある。

.. _sec-cfd-int-roles:

構成要素と役割
--------------

IgnisYeet の CFD モードは、次の外部プログラムを呼び出して、形状の準備、メッシュ、(Mach 数, 迎角, 横滑り角) の各ケースの実行、後処理、係数表の作成までを自動で行う。

* `SU2 <https://su2code.github.io/>`__ :cite:`su2`：流れの解法。Euler 方程式と RANS 方程式を解く。
* `gmsh <https://gmsh.info/>`__ :cite:`gmsh`：体積メッシュの生成。
* `OpenCASCADE <https://dev.opencascade.org/>`__ :cite:`occt`：CAD の立体カーネル。gmsh から使い、``panel_mesh`` の形状の再構成、``step`` の CAD のファイル（STEP、IGES、BREP）の読み込み、遠方境界の切り抜きを行う。
* `Manifold <https://github.com/elalish/manifold>`__ :cite:`manifold`：重なった閉じた STL の殻のブーリアン和。``surface = "stl"`` のときだけ使う。
* Open MPI :cite:`openmpi`：1 ケースを複数のランクで並列に解くときに使う。

:numref:`fig-cfd-pipeline` に全体の流れを示す。
壁面の形状は、``surface = "stl"`` では入力の STL を修復して一つの立体にしたもの、``surface = "panel_mesh"`` では抽出した半径分布とフィンの寸法から作り直した立体、``surface = "step"`` では CAD のファイルの立体である。
どれも gmsh で体積メッシュにし、SU2 で各ケースを解き、力とモーメントの係数を機体軸の係数に直して、他の手法と同じ形式の係数表にする。

.. _fig-cfd-pipeline:

.. figure:: _generated/tikz/cfd_pipeline.*
   :width: 100%

   CFD モードの処理の流れ。橙は gmsh、OpenCASCADE、Manifold を呼ぶ処理（Python）、青は Rust 側の処理（STL の修復、SU2 の実行と後処理）である。
   STL の修復（頂点の溶接、面の向きの統一、水密の確認）は Rust で行い、重なった殻の和だけを Python（Manifold）で求める。
   STL モードの内訳は :numref:`fig-cfd-stl-pipeline` に示す。

支配方程式
----------

本節は、SU2 に解かせる方程式を示す。既定は非粘性の Euler 方程式で、RANS は乱流を渦粘性で表す Spalart–Allmaras モデルを足したものである。

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
:math:`c_{b1},c_{b2},c_{w1},\sigma` はモデル定数、:math:`f_{v1},f_w` は壁近傍の減衰関数である。
SU2 の SA モデルを、:math:`\tilde\nu` が負になったときにも解が壊れない負の値の扱い（``NEGATIVE``）と、遷移の項 :math:`f_{t2}`\ （``WITHFT2``）を付けて使う（``SA_OPTIONS= ( NEGATIVE, WITHFT2 )``。SU2 8.3 は ``NEGATIVE`` だけの指定を受け付けない）。
壁面（``wall`` と ``base``）は断熱の粘着壁（``MARKER_HEATFLUX`` で熱流束 0）である。
``yplus`` が 5 以下（既定は 1）では、最初の層を粘性底層に置いて壁面まで境界層を解像し、壁関数は使わない。
``yplus`` が 30 から 150 のときだけ、対数則に基づく壁関数（SU2 の ``STANDARD_WALL_FUNCTION``）を ``wall`` と ``base`` に使う。その間の値は、どちらの扱いも成り立たないのでエラーにする。
自由流の渦粘性は :math:`\tilde\nu=3\nu`\ （``FREESTREAM_NU_FACTOR = 3``）である。

SU2 の設定と数値解法
--------------------

本節は、IgnisYeet が SU2 に渡す離散化と時間積分の設定、生成される設定ファイル、座標と符号の取り決めを述べる。

.. _sec-cfd-int-discretization:

離散化
~~~~~~

SU2 は非構造メッシュの節点を中心とする有限体積法で、各節点のまわりの双対セルについて\ :eq:`eq-cfd-euler` を離散化する。
IgnisYeet が Euler に設定する数値解法は次のとおりである。

* **対流フラックス**\ ：既定は Roe の近似 Riemann 解法 :cite:`roe`\ 。``scheme = "jst"`` では Jameson–Schmidt–Turkel の中心差分と人工粘性 :cite:`jst`\ （センサ係数 0.5 と 0.02）を使う。
* **2 次精度化**\ ：Roe のときは MUSCL 形の再構成（勾配は重み付き最小二乗法）に Venkatakrishnan の制限関数 :cite:`venkat`\ （係数 0.05）を組み合わせる。
  ``min_iterations``\ （既定 100）反復の時点で制限関数の値を固定する。固定しないと、制限関数の切り替わりで残差が 10\ :sup:`-4` 付近から下がらなくなるためである。
  固定の反復は早期終了の最小反復数（:ref:`sec-cfd-int-convergence`）と同じにしてあり、残差が下がる余地を残して止める。
  JST では再構成を使わない。
* **時間積分**\ ：定常解を得るために、陰的 Euler 法による擬似時間の進行を使う。連立 1 次方程式は FGMRES（前処理は ILU、許容 10\ :sup:`-6`、10 反復）で解く。
  CFL 数は適応的に変え、既定の初期値 ``cfl`` = 5 から、収束が進むと 1.2 倍に増やし、悪化すると 0.5 倍に減らす。範囲は :math:`0.2\,\mathrm{cfl}` から :math:`50\,\mathrm{cfl}` である。
* **収束判定**\ ：密度の二乗平均残差が ``convergence``\ （既定 :math:`10^{-6}`）以下になるか、``iterations``\ （既定 3000）に達すると終了する。判定は 10 反復目から行う。
* **マルチグリッド**\ ：``multigrid``\ （``MGLEVEL``、既定 0）を 1 から 4 にすると、SU2 の凝集型マルチグリッドの設定（``MGLEVEL``、``MGCYCLE``）を書く。
  サンプルのメッシュでは SU2 が粗い格子を作れないので、効果がない（:ref:`測定 <sec-cfd-multigrid>`）。

RANS の数値解法は次のとおりである（``scheme = "roe"`` のとき）。

* **対流フラックス**\ ：Roe の近似 Riemann 解法。冷えた状態から始めるケースは、まず 1 次精度（再構成なし）で解き、その解から 2 次精度で解き直す（二段の計算。:ref:`sec-cfd-int-rans-stages`）。
* **2 次精度化**\ ：MUSCL 形の再構成に、辺ごとの Van Albada の制限関数（``VAN_ALBADA_EDGE``）を組み合わせる。制限関数は固定しない（``LIMITER_ITER`` を書かない）。
  Venkatakrishnan–Wang の制限関数では、3 次元のケース（サンプル機体の :math:`M=0.8`、円すいの :math:`M=2`）の残差が CFL の上下とともに 10\ :sup:`-4` と 10\ :sup:`-6` の間を行き来し、制限関数を固定すると発散したためである。
* **勾配**\ ：粘性項と乱流量の勾配は Green–Gauss 法、再構成の勾配は重み付き最小二乗法で求める。
* **乱流量**\ ：SA の対流は 1 次風上（``SCALAR_UPWIND``、再構成なし）、時間積分は陰的 Euler 法。
* **時間積分**\ ：陰的 Euler 法による擬似時間の進行で、Roe の正確なヤコビ行列（``USE_ACCURATE_FLUX_JACOBIANS``）を使う。
  連立 1 次方程式は FGMRES（前処理は ILU(0)、相対許容 0.05、25 反復まで）で解く。各ステップの線形系を緩く解くことで、CFL 数が上限まで増える。
  CFL 数は初期値 ``cfl`` から Euler と同じ規則で適応的に変え、範囲は :math:`0.2\,\mathrm{cfl}` から :math:`40\,\mathrm{cfl}` である。
* **収束判定**\ ：SU2 には密度と SA の残差（``RMS_DENSITY``、``RMS_NU_TILDE``）が 10\ :sup:`-14` に下がるまで解かせ、実際の停止はランナーが受け入れの判定（:ref:`sec-cfd-int-rans-accept`）で決める。反復数の上限は ``iterations``\ （RANS の既定 4000）で、2 次精度の段で数える。
* **出力**\ ：履歴には残差、SA の残差（``rms[nu]``）、相対残差、力の係数、マーカごとの力の係数（``AERO_COEFF_SURF``\ 。``CFx(wall)``、``CFx(base)`` など）、線形解法の反復数、CFL 数を書く。
  解と壁面の分布（``restart.dat``、``surface_flow.vtk``\ 。壁面の :math:`y^+` を含む）は 50 反復ごと（``OUTPUT_WRT_FREQ= 50``）に書く。SU2 は SIGTERM を受けると何も書かずに終わるので、ランナーはケースを止める前に次の書き出しを待つ（:ref:`sec-cfd-int-rans-accept`）。
* **マルチグリッド**\ ：使わない（``multigrid`` の既定は 0）。

生成される設定ファイルと基準量
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

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

RANS の 2 次精度の段（半モデル、``cfl`` = 5、``yplus`` = 1）では、上と異なる主な行は次のとおりである。
1 次精度の段は ``MUSCL_FLOW= NO`` で、``ITER`` は ``iterations`` と 500 の小さいほう、``CONV_FIELD= REL_RMS_DENSITY``、``CONV_RESIDUAL_MINVAL= -2``、``CONV_STARTITER= 100`` とし、解を ``restart_startup.dat`` に書く。

.. code-block:: text

   SOLVER= RANS
   KIND_TURB_MODEL= SA
   SA_OPTIONS= ( NEGATIVE, WITHFT2 )
   RESTART_SOL= YES
   SOLUTION_FILENAME= restart_startup.dat
   VISCOSITY_MODEL= SUTHERLAND
   FREESTREAM_NU_FACTOR= 3.0
   MARKER_HEATFLUX= ( wall, 0.0, base, 0.0 )
   MARKER_MONITORING= ( wall, base )
   NUM_METHOD_GRAD= GREEN_GAUSS
   NUM_METHOD_GRAD_RECON= WEIGHTED_LEAST_SQUARES
   CONV_NUM_METHOD_FLOW= ROE
   MUSCL_FLOW= YES
   SLOPE_LIMITER_FLOW= VAN_ALBADA_EDGE
   USE_ACCURATE_FLUX_JACOBIANS= YES
   CONV_NUM_METHOD_TURB= SCALAR_UPWIND
   CFL_ADAPT_PARAM= ( 0.5, 1.2, 1.000, 200.0 )
   LINEAR_SOLVER_ILU_FILL_IN= 0
   LINEAR_SOLVER_ERROR= 0.05
   LINEAR_SOLVER_ITER= 25
   ITER= 4000
   CONV_FIELD= ( RMS_DENSITY, RMS_NU_TILDE )
   CONV_RESIDUAL_MINVAL= -14
   HISTORY_OUTPUT= ( ITER, RMS_RES, REL_RMS_RES, AERO_COEFF, AERO_COEFF_SURF, LINSOL, CFL_NUMBER )

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
符号は、軸対称な表の展開（:eq:`eq-coeff-6-axisym`）との一致で確かめた（:ref:`ロールと符号の検証 <sec-cfd-val-roll>`）。
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

形状の準備
----------

本節は、壁面になる立体をどう作るかを述べる。``panel_mesh`` では半径分布とフィンの寸法から立体を作り直し、``stl`` では入力の STL を修復して重なった殻を一つの立体にし、``step`` では CAD のファイルの立体を読む。
``panel_mesh`` と ``stl`` の CFD の壁面は入力の形そのものではなく、``stl`` ではずれの大きさを形状の忠実度で測る。``step`` では CAD のモデルを ``rocket.stl`` と照合する。

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
一方で、形状は回転体と台形フィンに限られ、レールボタンやポッド、非対称な突起は入らない。それらを含めて解くときは STL モードか、CAD のモデルを使うモード（``step``）を使う。

パネル法の表面メッシュを直接使わないのは、その三角形がノーズ先端やフィンの先端が胴体と交わる付近で極端に小さくなり、
gmsh の離散曲面からの四面体分割が「頂点が線分上にある」というエラーで失敗するためである。
機体の底面は、Euler では尾部フェアリング（:ref:`sec-cfd-fairing`）、RANS では平らな底面のままにする。

RANS では、フィンの前縁と後縁を少し鈍くする。両凸断面の厚さの分布 :math:`4\xi(1-\xi)` のうち :math:`\xi\in[0.02,\,0.98]` の部分を翼弦の全長に引き伸ばし、前縁と後縁を短い直線で閉じる。
縁の厚さは最大の厚さの約 8 % になる。鋭い縁ではプリズム層の柱が折れ重なるためである。Euler の形状は変えない。

STL の処理
~~~~~~~~~~

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
   和の表面は、:math:`y=0` と :math:`z=0` の平面について鏡映対称かを検査する（:ref:`sec-cfd-stl-symmetry-check`）。
5. **壁面の再メッシュと体積メッシュ**\ 。``stl_remesh`` による（下記）。対称と確かめた平面があれば、和をその平面で切って 1/4 領域または半領域だけをメッシュにし、そのあと鏡映して必要な領域にする。対称でなければ遠方境界は箱の全領域である。
6. **形状の忠実度**\ （:ref:`sec-cfd-int-fidelity`、結果の読み方は :ref:`sec-cfd-fidelity`）。

``stl_remesh`` は、和の表面を体積メッシャに渡す前の処理を選ぶ。

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - 値
     - 内容
   * - ``"classify"``\ （既定）
     - gmsh の ``classifySurfaces``\ （折れ角 40 度）と ``createGeometry`` で曲面を作り、壁面の大きさ ``wall_size`` で表面を作り直す。
       STL は曲面を細長い三角形に、平らな面（フィン）を少数の大きな三角形に分けるので、等方的な三角形に置き換える。曲面の再構成に失敗したときはエラーにする。
       要素の大きさは :eq:`eq-cfd-size` と同じで（フィンの面は :eq:`eq-cfd-size-fin`）、曲率の大きい部分は円周あたり 12 要素にし、最小の大きさは :math:`h_w/5` と :math:`h_f` の小さいほうまでとする（これより細かい曲率の要素は、再メッシュした面に HXT が受け付けない反転した三角形を作る）。``panel_mesh`` の曲率の設定（円周あたり 24 要素、最小 :math:`h_w/8`）とは別である。
   * - ``"none"``
     - 和の三角形をそのままメッシュにする。サンプルの STL ではフィンの面が少数の大きな三角形で、フィンを解像できず、法線力が 3 割ほど低くなる。


.. _sec-cfd-step-internals:

CAD のモデル（STEP）の処理
~~~~~~~~~~~~~~~~~~~~~~~~~~

``surface = "step"`` は、CAD のファイルを OpenCASCADE の B-rep（境界表現の立体）として読み、その曲面をそのまま壁面にする（使い方は :ref:`sec-cfd-step`）。
処理は Rust 側（単位、位置合わせ、対称性の判定、キャッシュ）と、gmsh の Python で動くスクリプト ``mesh_step.py`` の二つの段（``prepare`` と ``mesh``）に分かれる。

1. **単位**\ 。gmsh の ``Geometry.OCCTargetUnit = "M"`` で、STEP と IGES の長さをファイルが宣言する単位から m に換算して読む。BREP は単位を持たないので m として読む。
   ``step_unit`` を指定したときは、IgnisYeet がファイルの宣言（STEP の ``LENGTH_UNIT``\ 。``SI_UNIT`` と接頭語、または ``CONVERSION_BASED_UNIT('INCH')`` などの名前。IGES はグローバル部の 14 番目の単位の番号）を読み、
   読み込んだ形状を ``step_unit`` / 宣言の単位 の倍率で拡大する（BREP は宣言の単位を 1 m とする）。宣言を読めない STEP や IGES で ``step_unit`` を指定するとエラーにする。
2. **準備**\ （``prepare``）。ファイルを読み、ソリッドがなければ面を縫い合わせて立体にし（許容は全長の :math:`10^{-6}` 倍）、複数のソリッドは融合して一つにする（離れたままのソリッドは残して報告する）。
   単位の倍率をかけ、STL と同じ CFD の座標への変換（:ref:`sec-cfd-stl` の手順 1。機体の基底と原点から作るアフィン変換で、行列式は 1、鏡映しない）を当てる。
   次に表面を三角形に分割する。三角形の大きさは :math:`2h_w` を :math:`L/1000` から :math:`L/300` の範囲に収めた値で、最も長い辺の 1/300 より小さくはしない（単位を誤って 1000 倍の形状を読んでも、すぐに終わって位置合わせで単位の誤りを示せる）。
   表面メッシャがこの面を分割できないときに限り、OpenCASCADE の形状の修復（縮退した辺、小さい辺と面）をかけて読み直す。修復は常にはかけない。回転体の正しい立体では、修復がノーズ先端の縮退した辺を消し、そのあと gmsh が表面を分割できなくなるためである。
   面ごとに、種類、面積、重心、法線（面の内側の 5 × 5 点の法線がそろえば平面とみなす。平面を B-spline で書き出した面も平面になる）、包含箱、幅 :math:`2A/P`\ （面積 :math:`A`、周長 :math:`P`。帯の幅、円板の半径にあたる）を記録する。
   最も小さい幅を、最も細い特徴の大きさとする。
   結果（準備した形状 ``step_prepared.brep``、三角形分割 ``step_surface.stl``、報告 ``step_report.json``）は、ファイルの内容と準備の設定と ``mesh_step.py`` のハッシュでキャッシュする。
3. **位置合わせ**\ 。Rust 側で、CFD の座標に移した ``rocket.stl`` と、CAD のモデルの三角形分割を比べる（基準は :ref:`sec-cfd-step`）。
   まず包含箱を比べる。単位や向きの大きな誤りは、ここで距離を測らずに止める（1000 倍の面の上で距離を探すと非常に遅いため）。
   次に、STL の面から面積に比例して選んだ 4 万点（:ref:`sec-cfd-int-fidelity` と同じ標本の取り方）から CAD のモデルの三角形への最短距離を求める。
   CAD のモデルの内側にある点は、:math:`+x` 方向の半直線と三角形の交点の数の偶奇で判定し、:math:`\max(h_w,\,0.002L)` より深い点を埋まった殻の点として除く。
   逆向き（CAD のモデルの三角形の重心から STL への距離）は合否に使わず、:math:`\max(h_w,\,0.002L)` より離れた三角形を、頂点を共有するものどうしでつないで CAD の細部の領域として数える。
   合わないときの原因の推定は、両方の包含箱の最も長い辺の比による。比が単位の比（1000、25.4、10、100、304.8 とその逆数など）の 5 % 以内なら単位、1 の 5 % 以内なら向きか位置、それ以外は別の版のモデルとする。
4. **対称性**\ 。準備の段で、三角形分割の面の内側の節点（どれも B-rep の曲面の上にある）を、面ごとの節点の数に比例して選び（合計約 6000 点、各面 8 点以上）、:math:`y=0` と :math:`z=0` の平面について鏡映する。
   鏡映した点から、包含箱が近い面の曲面への最寄りの点を gmsh で厳密に求め、その点が面の境界の内側にあり、距離が許容以内なら一致とする。すべての点が一致すれば、その平面について対称である。
   許容は、全長の :math:`10^{-6}` 倍と、鏡映する前の節点を自分の面に射影し直したときの誤差の最大値の 4 倍の、大きいほうである。
   射影し直しの誤差は B-spline の曲面の精度で、どの鏡映の検査もこれより細かくは判定できない。Basic Finner の STEP（全長 0.3 m）では、この誤差（約 :math:`6\times10^{-7}` m）が全長の :math:`10^{-6}` 倍（:math:`3\times10^{-7}` m）を超えていた。
   それでもランチラグやレールボタンのずれ（mm の大きさ）よりはるかに小さいので、非対称な形状は対称と判定されない。
   一致しない点があれば、その数と最も離れた点の距離と位置を理由として記録する。三角形分割どうしの比較は行わない（``triangulation_identical`` は常に偽）。
   判定（:ref:`sec-cfd-stl-symmetry`）は ``stl`` と同じで、RANS では 1/4 領域を使わない。
   1/4 領域（:math:`y\ge0`、:math:`z\ge0`）を切り出すブーリアン演算は、ノーズ先端を通る :math:`y=z=0` の辺を作るので、読み込んだ形状では OpenCASCADE が失敗することがある（壁面が残らない）。
   そのときは半領域をメッシュにし、失敗したことを準備した形状のハッシュとともに ``step_quarter_failed.json`` に記録して、次からは最初から半領域にする。
   半領域のメッシュは :math:`z` について厳密に対称でないので、``alpha_mode = "mirror"`` では負の迎角も解く。
5. **底面**\ 。面の重心と法線から、STL と同じ規則で底面を決める。重心が底面の平面 :math:`x=L` より :math:`10^{-4}L` を超えて後ろにある面（フェアリング、突き出たノズル）と、その範囲内にあって平面で :math:`|n_x|>0.9` の面（平らな底面、底面にそろったフィンの後縁）が ``base``、それ以外が ``wall`` である。
   Euler の ``"fairing"`` は、:math:`x=L` の範囲内にある平らな底面のうち最も面積の大きい面の後ろに、OpenCASCADE の円すいを融合する。
   円すいの底の半径は、その面の包含箱から求めた底面の半径 :math:`r_b`、長さは :math:`\ell=` ``tail_fairing`` :math:`\times r_b` の先の一点に向かう傾きで、長さ :math:`0.9\ell`\ （半径 :math:`0.1r_b`）で切って平らな面で閉じる。
   ``panel_mesh`` と ``stl`` のフェアリングと同じ傾きである。尖ったままにしないのは、OpenCASCADE の円すいの頂点に、gmsh のどの表面メッシャも重複した三角形か縮退した三角形を残すためである。
6. **メッシュ**\ （``mesh``）。Euler では、遠方境界の領域（全領域は箱、半領域は半球、1/4 領域は球の 1/4。:ref:`sec-cfd-domain`）から準備した立体を切り抜き、四面体で埋める。
   大きさは :eq:`eq-cfd-size` で、フィンの面は :eq:`eq-cfd-size-fin` による（フィンの面の選び方は「メッシュの大きさ」の項にある）。
   曲率の大きい部分は円周あたり 24 要素にし、最小の大きさは
   :math:`h_{\min}=\operatorname{clamp}(w_{\min}/4,\;h_w/8,\;h_w)`\ （:math:`w_{\min}` は最も小さい面の幅）とする。``sample_rocket.step`` のランチラグの管の壁（幅約 0.8 mm）は、``wall_size`` 2 mm で最小の大きさ 0.25 mm になり、幅の方向に 3 個ほどの要素が並ぶ。
   表面のメッシュを作ったあと、面ごとに重複した三角形と縮退した三角形を探し、あればその面だけを Delaunay 法、次に MeshAdapt 法で作り直す（CAD から書き出した回転体のノーズは、周期的な面の頂点にこの三角形が残ることがあり、HXT が ``duplicated facets`` で失敗する）。
   体積は HXT 法で、失敗したら新しいプロセスで Delaunay 法でやり直す。1/4 領域は :math:`z=0` で、必要なら半領域を :math:`y=0` で鏡映する（:ref:`sec-cfd-stl-symmetry-check` と同じ）。
   RANS は手順 7 による。
7. **プリズム層（RANS）**\ 。:ref:`sec-cfd-prism` と同じ層の規則（最初の層の高さ、層数、厚さの上限、底面の柱、層の上面からの四面体）で、共通のスクリプト ``bl_layer.py`` が層を積む。
   半領域は :math:`y\ge0` の半分の箱で作る。全領域は、形状が :math:`y=0` について対称なら半領域を鏡映し、対称でなければ閉じた壁面のまわりの箱に直接作る。CAD の縁は鈍くしない。
   ``step`` では、狭いすき間のための規則（すき間の扱い）を加える。

   * 壁面の各節点から法線の方向に半直線を伸ばし、層の厚さの 2.5 倍以内で再び壁面に当たれば、柱の厚さをその距離の 0.4 倍以下にする（ランチラグの穴、レールボタンのつばの下）。
   * 柱の 8 つの高さの点を比べ、別の壁の柱の点が、局所の辺の長さの 0.3 倍より、かつ二つの壁の節点の距離の半分より近ければ、二つの柱が交わるとみなし、どちらも交わる低いほうの高さの 0.6 倍に切る。
     プリズムが反転しなくなったあとにこの検査を行い、切った柱があれば反転の検査からやり直す。
   * 層数は変えず、薄い柱は最初の層の高さ :math:`h_1` の層を下に並べ、上の層だけを層の比で大きくする。最後の層が厚く残るので、外側の四面体との大きさの比が小さくなる。厚さが十分な柱は、通常の等比の高さのままである。
   * 層の上面の三角形ごとに、そこから始まる四面体に必要な大きさ（その下の最後の層の高さの 2 倍。面の大きさの 1/3 より小さくすると細長い四面体になるので、それより小さくはしない）を求める。
     これが上面の四面体の大きさ ``top_size`` の半分より小さい三角形は、``top_size`` を半分ずつにした大きさの組に分けて別の面にし、組ごとに大きさの場を置く。

   これらの規則は ``step`` だけで使い、``panel_mesh`` の RANS のメッシュは変わらない。
   層の検査（:ref:`sec-cfd-prism` の「検査と書き出し」）は同じで、作り直しても検査に通らなければ、鋭い縁を丸めるか Euler を使うよう示すエラーにする。
   層の上面での大きさの比（``bl_top_ratio_p99`` と ``bl_top_ratio_max``）は ``mesh_stats.json`` に記録する。``sample_rocket.step`` の RANS のメッシュの試験は、99 パーセンタイルが 8 未満であることを確かめる。

メッシュのハッシュは、CAD のファイルの内容、両方の段の設定（パスとスレッド数を除く）、``mesh_step.py``、``bl_layer.py``、``su2_writer.py`` から作る。
準備の段のハッシュはメッシュの設定を含まないので、``wall_size`` などを変えても読み込みと対称性の検査はやり直さない。
読み込みの経過は ``prepare.log``、メッシュの経過は ``mesh.log`` に残る。

.. _sec-cfd-stl-symmetry-check:

鏡映対称の検査と 1/4・半領域のメッシュ
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

``surface = "stl"`` は、和の表面が鏡映対称と確かめられたときだけ、1/4 領域または半領域をメッシュにする。
本節はその検査の手順と、検査に通った形状のメッシュの作り方を述べる。利用者から見た動作（``shape_symmetry`` の値と領域）は :ref:`sec-cfd-stl-symmetry` にある。

和の表面を、機軸を含む二つの平面 :math:`y=0`\ （ピッチ面）と :math:`z=0` について別々に検査する。
検査は、位置の許容を全長の :math:`10^{-6}` 倍（サンプルで 1.8 μm）として、次の二段で行う。

1. 頂点を許容で溶接し、溶接した頂点をすべて鏡映する。鏡映した点に頂点がなければ非対称とする。
   鏡映した三角形の集合（ソートした頂点の三つ組）が元の集合と一致すれば、そこで対称と判定する。
2. 三角形の分割が鏡映の関係にないときは、表面そのものを比べる。鏡映した三角形の頂点、辺の中点、重心がすべて、元の表面から許容以内にあれば対称とする（最寄りの三角形を厳密に探す）。
   鏡映した面は、向きの違う対角線で分割した平らな四角形と同じ点の集合になる。メッシュ生成器が曲面や平面を鏡映の関係にない三角形に分割するのは普通のことで、
   サンプルの STL でも三角形の 99 % が鏡映のもの（頂点が同じ）と一致しない。
   頂点の一致は結果に記録するが、判定には使わない（三角形の分割が違えば、ブーリアン演算で生じる交線の頂点の位置が一致するとは限らない）。

平面ごとに、対称か否か、鏡映した点と表面の最大の距離（``max_mismatch``）、一致した頂点の数、三角形の分割が同一か、非対称のときの理由（最大の距離とその位置、対応する頂点のない頂点の数）を記録する。
サンプルの STL での検査の結果は :ref:`sec-vaero-cfd-symmetry` に示す。
尾部フェアリングの円すいは、胴体の底面の縁が両方の平面について対称で、縁の頂点が平面上にあるとき、頂点数が 4 の倍数の環を 1/4 の領域でつないで鏡映して作るので、分割まで厳密に対称である。
縁がそうでないときは、環を角度の順につなぐだけの作り方になり、和は検査を通らない。

検査の結果と ``shape_symmetry`` から対称性と領域が決まる（:ref:`sec-cfd-stl-symmetry`）。
選んだ対称性と理由は、実行のとき標準エラー出力に表示し、``stl_fidelity.json`` と ``cfd_report.json``\ （``symmetry_check``）に平面ごとの数値とともに記録する。メッシュのハッシュには選んだ領域が含まれる。

メッシュは、対称な平面で切った和の表面から作る。三角形を平面で切って（平面上の頂点は平面に載せる）、``union_clip.stl`` に書く。
切った表面は切断面に沿って開いていて、その境界の曲線が箱の対称面を閉じる。
gmsh の ``classifySurfaces`` は、境界の曲線を折れ角が 0.1 rad（約 6 度）を超える点で分けるので、切断面が機軸と交わる点（ノーズ先端、尾部の先端）が曲線の端点になる。
対称面（``symmetry`` が :math:`y=0`、``symz`` が :math:`z=0`）は、箱の辺と、この離散曲線を並べた閉じた輪でできる平面で、1/4 領域では輪が機軸に沿って箱の隅の点とノーズ先端、尾部の先端を結ぶ。
メッシュのあとは、CAD のメッシュと同じに仕上げる。1/4 領域は :math:`z=0` で鏡映して :math:`z` について厳密に対称な半領域のメッシュにし（頂点の順序を入れ替えて向きを保ち、:math:`z=0` 上の節点は共有する）、
半領域のままにするか（``symmetry`` のマーカを持つ SU2 の半領域）、:math:`y=0` で鏡映して全領域にする。内部の対称面のマーカ（``symz``、全領域では ``symmetry`` も）は落とす。
半領域のままにする条件は :ref:`sec-cfd-stl-symmetry` と同じである。
:math:`z` について厳密に対称なので、``alpha_mode = "mirror"`` では :math:`-\alpha` は :math:`+\alpha` の鏡像で、解かない（:ref:`sec-cfd-alpha-mode`）。

サンプルの STL で作った 1/4・半領域・全領域のメッシュのセル数と作成時間、鏡映した全領域と半領域の係数の比較は :ref:`sec-vaero-cfd-symmetry` に示す。
STL のメッシュは実行ごとに同一にならないので、半領域と全領域の比較は、同じメッシュから作ったものどうしでなければ意味を持たない。

.. _sec-cfd-int-fidelity:

形状の忠実度の測り方
~~~~~~~~~~~~~~~~~~~~

STL モードでは、CFD の壁面が入力の形からどれだけずれたかを測る。再メッシュとフェアリングで形が変わるので、結果の信頼性の目安になる。
利用者向けの読み方は :ref:`sec-cfd-fidelity` にある。ここでは何を比べ、どう測るかを述べる。
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
結果は ``<出力先>/<workdir>/mesh/stl_fidelity.json`` に保存される。
``cfd_report.json`` の ``fidelity`` にも、修復と和の報告とともに入る。距離はサンプリングによる近似で、細い特徴の見落としがありうる。

サンプルの STL を ``wall_size`` 2 mm で作った結果の読み方は :ref:`sec-cfd-fidelity` に、三つの面の比較と距離の表は :ref:`sec-vaero-cfd-fidelity` に示す。

メッシュ生成
------------

本節は、体積メッシュの領域、境界のマーカ、セルの大きさの決め方を述べる。
Euler の体積メッシュは四面体で、gmsh の HXT 法で作る。HXT が失敗したときは Delaunay 法でやり直す（``panel_mesh`` は同じプロセスで、``stl`` と ``step`` は新しいプロセスで行う）。
RANS のメッシュは、壁面と底面の上のプリズム層と、その外側の四面体からなる（:ref:`sec-cfd-prism`）。

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
     - 1/4 または半分の箱を作り、``panel_mesh`` と同じに鏡映して半領域または全領域にする（:ref:`sec-cfd-stl-symmetry-check`）
   * - ``stl``、対称でない（または ``shape_symmetry = "none"``）
     - 一辺 :math:`2\times` ``farfield`` × 全長の箱の全領域（:ref:`sec-cfd-stl`）
   * - ``step``、Euler
     - 対称と確かめた平面に応じて、球の 1/4、半球、または箱の全領域。1/4 と半分は ``panel_mesh`` と同じに鏡映して半領域または全領域にする（:ref:`sec-cfd-step-internals`）
   * - RANS（``panel_mesh`` と ``step``）、半領域
     - 半分の箱（:math:`y\ge0`、機体の中央を中心とする一辺 :math:`2\times` ``farfield`` × 全長の立方体の半分）。:math:`y=0` の平面は ``symmetry``
   * - RANS、全領域、``panel_mesh`` でフィンの配置が :math:`y=0` について対称、または ``step`` で形状が :math:`y=0` について対称
     - 半分の箱のメッシュを :math:`y=0` で鏡映した箱。``panel_mesh`` で対称でない配置はエラーにする（:ref:`sec-cfd-rans-use`）
   * - RANS、全領域、``step`` で形状が :math:`y=0` について対称でない
     - 閉じた壁面のまわりの箱の全領域に、直接プリズム層と四面体を作る

半領域を使う条件と ``shape_symmetry`` の値と動作は :ref:`sec-cfd-stl-symmetry` にある。
形状が :math:`y=0` について対称かどうかは、``panel_mesh`` ではフィンの配置から、``stl`` では和の表面の検査で、``step`` では CAD の曲面の検査で決まる。

箱の遠方境界は、機体の中央を中心とする一辺 :math:`2\times` ``farfield`` × 全長の立方体である。
箱の 6 つの面は辺（曲線）を共有させて作る。共有しないと、HXT が「頂点が線分上にある」というエラーで失敗する。
対称と確かめた STL では、箱ではなく 1/4 または半分の箱を使う（:ref:`sec-cfd-stl-symmetry-check`）。

境界には名前（マーカ）が付いている。

.. list-table::
   :header-rows: 1
   :widths: 20 80

   * - マーカ
     - 内容
   * - ``wall``
     - 機体表面（胴体とフィン）。力を積分する面。STL モードでは、再メッシュした和の表面のうち ``base`` でない三角形。``step`` では、CAD の面のうち ``base`` でない面
   * - ``base``
     - 底面。Euler では尾部フェアリングの面（STL で ``"flat"`` なら平らな底面）で、力を積分しない。RANS では平らな底面の粘着壁で、力を履歴に出す（軸力に使うかは ``rans_base``\ 。:ref:`sec-cfd-post`）
   * - ``farfield``
     - 遠方境界。特性量に基づく非反射の境界条件
   * - ``symmetry``
     - :math:`y=0` の平面。半領域のときだけ。RANS では、プリズム層の側面の四角形も含む
   * - ``symz``
     - :math:`z=0` の平面。1/4 領域をメッシュにする途中だけにあり、鏡映して半領域にするときに落とす

STL モードの ``wall`` と ``base`` は、メッシュの三角形の重心と法線から分ける。
重心が底面の平面 :math:`x=L` より後ろに全長の :math:`10^{-4}` 倍以上あるか、その範囲内にあって法線が :math:`+x` を向く三角形（平らな底面と、底面に揃ったフィンの後縁）が ``base`` である。
``step`` では、同じ規則を CAD の面ごとに（面の重心と、平面かどうかと、法線で）当てる。

.. _sec-cfd-fairing:

尾部フェアリング
~~~~~~~~~~~~~~~~

非粘性の流れでは、平らな底面の後ろの流れは問題として成り立たない。
後流が非対称な定常解を持ち、解の対称性が保てないためである。
そこで Euler では、底面の後ろに円すい形の尾部フェアリングを置く。長さは ``tail_fairing``\ （既定 6）× 底面半径で、先端は一点に閉じる。
フェアリングの面は ``base`` マーカで、力の積分から外す。
したがって底面の抗力は CFD からは得られず、部品積み上げ法の底面抗力の項で補う（:ref:`sec-cfd-post`）。
RANS では後流が解の一部なので、底面は平らなままにする。
STL モードと ``step`` では ``base_treatment = "fairing"`` がこれに当たる（:ref:`sec-cfd-stl-base`）。

z 対称メッシュ
~~~~~~~~~~~~~~

機体が :math:`z` 方向にも鏡映対称（:math:`z\to-z`）であっても、非構造のメッシュは対称にならない。
半モデルのメッシュを一度に作ると、:math:`z>0` と :math:`z<0` でセルの並びが異なり、:math:`\alpha=0` でも法線力が 0 にならない（数値は :ref:`sec-vaero-cfd-symmetry` に示す）。
そこで ``z_mirror_mesh``\ （既定 ``true``）では、:math:`y\ge0` かつ :math:`z\ge0` の **1/4 領域だけ** をメッシュにし、
それを :math:`z=0` 面で鏡映して半球のメッシュにする（:numref:`fig-cfd-domain`）。
鏡映したセルは頂点の順序を入れ替えて、向きを正に保つ。:math:`z=0` の面上の節点は共有される。
こうして得た半モデルのメッシュは :math:`z` について **厳密に対称** で、:math:`-\alpha` の解は :math:`+\alpha` の解の鏡像になる（:ref:`sec-cfd-alpha-mode`）。

.. _fig-cfd-domain:

.. figure:: _generated/tikz/cfd_domain.*
   :width: 100%

   計算領域の概念図。(a) 半球の :math:`y=0` 面での断面。機体は拡大して描き、実際の遠方境界は機体よりはるかに大きい。
   上半分（:math:`z\ge0`）だけをメッシュにし、:math:`z=0` で鏡映して下半分を作る。
   (b) 機軸の方向から見た半球。:math:`y=0` が symmetry の平面で、フィンが :math:`+y` と :math:`\pm z` に付いている。

この手法が使えるのは、形状が :math:`z\to-z` について対称なときだけである（``panel_mesh`` では、フィンが 4 枚で ``fin_roll_deg`` が 0 度または 45 度、2 枚、6 枚など。3 枚や 5 枚、roll が 10 度などは非対称）。
対称でない形状では、``z_mirror_mesh`` は自動的に無視される。
``panel_mesh`` では、フィンの配置が :math:`z\to-z` について対称なときに ``z_mirror_mesh`` が働く。``stl`` では、和の表面が :math:`z=0` と :math:`y=0` について鏡映対称と確かめられたとき（:ref:`sec-cfd-stl-symmetry-check`）に、``step`` では CAD の曲面が両方の平面について鏡映対称と確かめられ、1/4 領域を切り出せたとき（:ref:`sec-cfd-step-internals`）に働く。
全領域でも、1/4 領域を :math:`z=0` と :math:`y=0` で鏡映して全領域にする。
RANS のメッシュは半領域から作るので、``z_mirror_mesh`` は効かない。``alpha_mode = "mirror"`` では負の迎角も解く。

メッシュの大きさ
~~~~~~~~~~~~~~~~

体積メッシュは四面体である（gmsh の HXT 法）。セルの大きさは、壁面までの距離 :math:`s` の関数

.. math::
   :label: eq-cfd-size

   h(s)=\min\bigl(h_{\max},\;h_w+0.2\,s\bigr),\qquad h_{\max}=\frac{R_\infty}{10}

で決める。:math:`h_w` は ``wall_size``\ （壁面の要素の大きさ、既定 2 mm）、:math:`R_\infty` は遠方境界の半径である。
壁から離れるにつれ、要素の列ごとに約 1.2 倍の割合で大きくなる。

フィンの面は ``fin_size`` :math:`h_f` で細かくする。フィンの面までの距離 :math:`s_f` について

.. math::
   :label: eq-cfd-size-fin

   h(s,s_f)=\min\bigl(h_{\max},\;h_w+0.2\,s,\;h_f+0.2\,s_f\bigr)

とし、フィンの面では :math:`h_f`、そこから同じ割合で大きくなって :math:`h_w` につながる。
``fin_size = "auto"`` の :math:`h_f` は、Euler で :math:`\min(h_w,\,t/4)`\ （:math:`t` はフィンの厚さ）、RANS で :math:`h_w` である。
:math:`h_f=h_w` のときはこの項を置かず、メッシュは :eq:`eq-cfd-size` だけで決まる。
フィンの面は、面の重心が胴体の外形（尾部フェアリングを含む）よりスパンの 1/10 以上外にある壁面として選ぶ。胴体の面は、半領域や 1/4 領域の扇形に切られていても重心が胴体の内側にあるので選ばれない。
``surface = "step"`` では CAD の面を、``stl_remesh = "classify"`` では ``classifySurfaces`` で作った面（STL の三角形の面積で重みをつけた重心）を同じ規則で選ぶ。ランチラグやレールボタンは胴体からの高さがスパンの 1/10 より小さければ選ばれない。
細かくする理由は :ref:`sec-cfd-val-asym-cause` にある。厚さより大きい要素でフィンを表すと、Euler の解には亜音速で :math:`\alpha=0` にも偽の法線力が残る。
RANS は、フィンの面を厚さの 1/4 にするとプリズム層の柱が増えすぎて 1 ケースのメモリに収まらず、:math:`\alpha=0` のずれも底面の後流によるもの（:math:`C_N(4^\circ)` の 1〜3 %）なので、``wall_size`` のままにする。
``panel_mesh`` では、曲率の大きい部分（ノーズ、フィンの縁）は、円周あたり 24 要素になるよう壁面の要素を細かくし、最小の大きさは :math:`h_w/8` と :math:`h_f` の小さいほうまでとする。``stl_remesh = "classify"`` の曲率の設定は円周あたり 12 要素、最小 :math:`h_w/5` で、モードごとに別である。
``step`` は円周あたり 24 要素で、最小の大きさは最も細い面の幅から決める（:math:`\operatorname{clamp}(w_{\min}/4,\,h_w/8,\,h_w)` と :math:`h_f` の小さいほう。:ref:`sec-cfd-step-internals`）。
メッシュの生成は単一スレッドで行う。複数スレッドでは実行のたびに異なるメッシュになり、CFD の結果の再現性が損なわれるためである。

既定の 2 mm（フィンの面は厚さ 4 mm の 1/4 で 1 mm）で、サンプル機体の半モデルは約 142 万セル（約 30 万節点）になる（計算量は :ref:`sec-cfd-run`）。
壁面（``wall``）は 23.4 万、尾部フェアリング（``base``）は 1.5 万、対称面は 3.3 万の三角形である。
フィンの面も 2 mm にすると（``fin_size = 0.002``）、約 94 万セル、壁面は 14.2 万の三角形である。

:numref:`fig-cfd-mesh-surface` に壁面のメッシュを示す。
ノーズ先端は曲率が大きいので、先端に向かって要素が細かくなる。フィンの付け根では、胴体とフィンの面が一つの立体として交わり、交線に沿って要素がつながる。

.. _fig-cfd-mesh-surface:

.. figure:: _generated/plots/cfd_mesh_surface.*
   :width: 100%

   サンプル機体の壁面のメッシュ（既定の ``wall_size`` 2 mm、フィンの面は ``fin_size = "auto"`` で 1 mm、半モデル）。どちらも :math:`+y` 側から機体の外側の面を見た図で、左はノーズ先端（先端から 9 cm。先端が右を向く）、右はフィンの付け根の前縁側。

体積メッシュの様子は、対称面（:math:`y=0`）での断面で分かる（:numref:`fig-cfd-mesh-section`）。
壁面の近くでは要素が 2 mm で、壁から離れるにつれ\ :eq:`eq-cfd-size` に従って大きくなり、遠方境界（半径 30 m）では最大の大きさ 3 m に達する。

.. _fig-cfd-mesh-section:

.. figure:: _generated/plots/cfd_mesh_section.*
   :width: 100%

   対称面 :math:`y=0` での体積メッシュの断面。左から計算領域の全体、機体のまわり、フィンと尾部フェアリングの拡大。灰色の部分は機体（とフィン）の内部である。メッシュは :numref:`fig-cfd-mesh-surface` と同じ（フィンの面は 1 mm）。

メッシュは ``<出力先>/<workdir>/mesh/`` に保存され、形状と設定のハッシュが同じなら再利用される。
ハッシュには、メッシュを作るスクリプト（SU2 のメッシュを書くスクリプトと境界層のスクリプトも）の内容と、``fin_size`` から決まるフィンの面の大きさが入る。

.. _sec-cfd-prism:

境界層のプリズム層（RANS）
~~~~~~~~~~~~~~~~~~~~~~~~~~

RANS のメッシュは、壁面の近くをプリズムの層で、その外側を四面体で埋める。
プリズム層は gmsh の境界層の機能ではなく、メッシュを作るスクリプトが共通の ``bl_layer.py`` で壁面の三角形から直接組み立てる。
本節は ``panel_mesh`` の手順を述べる。``step`` は同じ規則に、狭いすき間の規則を加える（:ref:`sec-cfd-step-internals`）。
gmsh は、壁面の三角形、外側の面の三角形、外側の四面体だけを作る。

**手順**\ 。

1. 作り直した立体（フィンの縁を鈍くしたもの。:ref:`sec-cfd-rebuild`）を :math:`y\ge0` の半分に切り、壁面（``wall``）と底面（``base``）の三角形を Euler と同じ大きさの規則（:eq:`eq-cfd-size`、曲率の大きい部分は円周あたり 24 要素、最小 :math:`h_w/8`）で作る。
2. 壁面の節点ごとに、層を積む向き（法線）と柱の厚さを決める（下記）。
3. 各柱に同じ数の層を積む。層の高さは最初の層の高さ :math:`h_1` から等比で増え、柱ごとに比を決めて、層の高さの和を柱の厚さに合わせる。
4. 層の上面の三角形、:math:`y=0` の平面（層の断面をくり抜いたもの）、半分の箱の遠方境界（:ref:`sec-cfd-domain`）で囲まれた領域を、gmsh で四面体に分ける（HXT 法。失敗したときは新しいプロセスで Delaunay 法でやり直す）。
   四面体の大きさは、層の上面からの距離 :math:`s` について :eq:`eq-cfd-size` と同じ :math:`\min(h_{\max},\,h_w+0.2\,s)` で、上面では ``wall_size`` から始まり、要素の列ごとに約 1.2 倍になる。
5. セルを検査して SU2 の形式で書く（下記）。全領域は、半領域を :math:`y=0` で鏡映して作る（:math:`y=0` 上の節点は共有し、``symmetry`` のマーカは落とす）。

**層の高さと数**\ 。最初の層の高さは、ノーズ先端から全長の 5 % の位置で :math:`y^+` が ``yplus`` になるように、平板の乱流の摩擦の式から決める。
解く最大の Mach 数 :math:`M_{\max}` と基準高度の大気で :math:`U=M_{\max}a`、:math:`x=0.05L` として

.. math::
   :label: eq-cfd-first-height

   C_f=0.0576\,\mathit{Re}_x^{-1/5},\qquad u_\tau=U\sqrt{C_f/2},\qquad h_1=\frac{y^+\nu}{u_\tau}

である。全長の位置より境界層の薄いノーズ側で決めるので、ほかの位置の :math:`y^+` は ``yplus`` より小さくなる。
``cfd_report.json`` の ``est_wall_yplus`` は、この :math:`h_1` を全長の位置で :math:`y^+=1` になる高さで割った値で、機体の後部での :math:`y^+` の見積もりである。
層の高さは 1.2 倍ずつ増やし、最後の層が :math:`1.3\,h_w` 以上になる最小の層数 :math:`n` で止める。厚さは :math:`T=h_1(1.2^n-1)/0.2` である。
最後の層の高さがその外側の四面体の大きさとほぼ同じなので、層の上面でセルの大きさが急に変わらない。
上面の四面体の高さと最後の層の高さの比（大きいほうを小さいほうで割った値）の 99 パーセンタイルと最大値は、``mesh_stats.json`` の ``bl_top_ratio_p99`` と ``bl_top_ratio_max`` に記録する。
厚さ :math:`T` には上限があり、機体の後端での平板の乱流境界層の厚さ :math:`0.37\,L\,\mathit{Re}_L^{-1/5}`\ （解く最小の Mach 数）の 1.5 倍と、胴体の最大半径の 0.5 倍の小さいほうを超えるときは、層の数を減らす。
そのときは、最後の層は上面の四面体より低い。
サンプル機体（``yplus = 1``、Mach 数 0.8 と 2）では、``wall_size`` 2.5 mm から 8 mm のどれでも厚さが上限で決まり、49 層、厚さ 21.7 mm、最後の層の高さ約 3.6 mm である（表は :ref:`sec-cfd-rans-use`）。

**法線**\ 。壁面の三角形の向きをそろえて外向き（流体の側）にする。一つの辺を 3 枚以上の三角形が共有するとき、向きをそろえられないときはエラーにする。

* 節点の法線は、まわりの三角形の法線との余弦の最小値が最も大きくなる向き（どの三角形からも最もよく見える向き）とする。
* :math:`y=0` 上の節点では、法線の :math:`y` 成分を 0 にして平面内に置き、:math:`y=0` で鏡映した三角形も含めて見える向きを求める。鏡映した全領域でも柱がそろう。
* 隣の節点の法線と平均して滑らかにする（3 回）。平均した向きが、まわりの三角形との余弦の最小値を元の値と 0.5 の小さいほうより下げるときは、その節点では平均しない。
* 凹んだ辺（隣り合う三角形が 30 度より大きく凹んで交わる辺。フィンと胴体の付け根）から層の厚さ以内の節点は、さらに :math:`2T/h_w` 回（四捨五入）平均して、隅から扇状に開く向きにする（隅肉のような形）。

**柱の厚さ**\ 。柱の厚さは :math:`T` を基本とし、次の制限をかける。層の数は変えないので、薄い柱では層の高さの比が小さくなる（最小でも :math:`n\,h_1`）。

* フィンと胴体の付け根の近くでは、二つの壁の柱が交わらないように、厚さを :math:`0.5\,T` に付け根の凹んだ辺までの距離を足した値以下にする。
* フィンの縁や先端のように隣の柱と向きが開く節点では、隣との法線の角度の最大値 :math:`\varphi` に対して厚さを :math:`2h_w/\varphi` 以下にし、上面の広がりを ``wall_size`` の 2 倍程度に抑える。
* 隣り合う柱の厚さの差を、辺の長さの半分以下にする。
* プリズムの 6 つの隅の Jacobian（隅の 3 辺の三重積）を求め、0 以下のものがあれば、その柱の厚さを 0.6 倍にして、まわりの差の制限をかけ直す。なくなるまで（最大 30 回）繰り返す。

**検査と書き出し**\ 。SU2 のメッシュは ``su2_writer.py`` が書く。セルは、符号付きの体積が負なら頂点の順序を入れ替えて向きをそろえ、SU2（VTK）の頂点の順序で書く。
セルは四面体（SU2 の型 10）とプリズム（13）、境界の面は三角形（5）と、:math:`y=0` にあるプリズム層の側面の四角形（9）である。
書く前に、隅の Jacobian が 0 以下のプリズム、体積が 0 以下のセル、どのマーカにも属さない境界の面、境界でないマーカの面を数える。
どれかがあれば、法線の平滑化を 10 回、付け根の厚さの係数を 0.25 にして、新しいプロセスで一度だけ作り直す。
それでも残れば、``mesh error: the prismatic boundary layer is invalid: … (try a different wall_size or yplus)`` でメッシュの作成を止める（``gmsh meshing failed (log: …)`` のエラーになる）。
何回目で通ったかは ``mesh_stats.json`` の ``bl_attempt`` に残る。SICN の品質は四面体だけについて求める。
サンプル機体の ``wall_size`` 2.5、3、3.5、4、4.5、5、6 mm と ``fin_roll_deg`` 0 度と 45 度のメッシュは、すべてこの検査を通る。

ケースの実行
------------

本節は、ケースの行列をどういう順序と初期値で解くか、収束とみなす条件、結果の再利用の規則を述べる。
行列の指定と計算資源の設定は :ref:`sec-cfd-run` にある。

.. _sec-cfd-alpha-mode:

迎角の正負と非対称性の補正
~~~~~~~~~~~~~~~~~~~~~~~~~~

``alpha_mode`` は、迎角の正負と :math:`\alpha=0` での非対称性の扱いを決める（値の一覧は :ref:`sec-cfd-run`）。

``mirror`` では、各正の迎角 :math:`\alpha` について、:math:`+\alpha` と :math:`-\alpha` の結果 :math:`f(\pm\alpha)` から

.. math::
   :label: eq-cfd-mirror

   C_N=\frac{C_N(+\alpha)-C_N(-\alpha)}{2},\qquad
   m=\frac{m(+\alpha)-m(-\alpha)}{2},\qquad
   C_A=\frac{C_A(+\alpha)+C_A(-\alpha)}{2}

を使う。:math:`m=C_N x_{cp}` はノーズ周りのモーメント（長さの次元）である。
正負の迎角で同じ値だけ現れる数値的なずれ :math:`c_0`\ （:math:`f(\pm\alpha)=\pm f_{odd}+c_0`）は、:eq:`eq-cfd-mirror` で打ち消される。
片方の迎角の解が得られなかったときは、もう一方に :math:`\alpha=0` の値をずれの推定値として使う。
:math:`\alpha=0` の解は一度だけ解き、軸力 :math:`C_A` だけを使う。その法線力とモーメント（真値は 0）は、計算で現れた非対称性として ``cfd_report.json`` の ``asymmetry`` に記録する。
``offset`` では、:math:`\alpha=0` の値を :math:`C_N^{(0)},m^{(0)}` として

.. math::
   :label: eq-cfd-offset

   C_N=C_N(\alpha)-C_N^{(0)},\qquad m=m(\alpha)-m^{(0)}

とする。``z_mirror_mesh`` が有効なとき、:math:`-\alpha` の解は :math:`+\alpha` の解の鏡像なので、``mirror`` の組合せは :math:`+\alpha` の結果に一致する。
:math:`\alpha=0` のずれは 0 になり、負の迎角の計算は不要になる。

どの符号のケースを解くかは ``shape_symmetry`` で決まり、一覧とケース数の数え方は :ref:`sec-cfd-run` にある。
解かない符号の係数は、後処理で鏡映して補う（:ref:`sec-cfd-general`）。

.. _sec-cfd-roll-cases:

ロール減衰のケース
~~~~~~~~~~~~~~~~~~

``roll_damping = true`` にすると、各 Mach 数で :math:`\alpha=\beta=0`、:math:`\hat p=\pm` ``roll_rate_hat`` の 2 ケースを回転座標系で解く（:ref:`sec-cfd-sideslip`）。
回転があると :math:`y=0` について対称でなくなるので、全領域で解く。Euler だけで使える。
2 ケースの差からロール減衰微係数 :math:`C_{lp}` を求める（:ref:`sec-cfd-roll-post`）。

暖機起動と並列実行
~~~~~~~~~~~~~~~~~~

各ケースは、近いケースの収束解を初期値にして始める（暖機起動）。元のケースは次のとおりである。

* :math:`\alpha\ne0`：同じ Mach 数と横滑り角で、一つ小さい迎角（符号が同じもの、最小は :math:`\alpha=0`）。
* :math:`\alpha=0`、:math:`\beta\ne0`：同じ Mach 数の :math:`\alpha=0` で、一つ小さい横滑り角（符号が同じもの、最小は :math:`\beta=0`）。
* :math:`\alpha=\beta=0`：一つ小さい Mach 数の :math:`\alpha=\beta=0`。
* ロール減衰のケース：同じ Mach 数の回転しない :math:`\alpha=\beta=0`。

元のケースが終わる（成功でも失敗でも）まで待ち、失敗した場合は自由流から始める。
RANS では、Mach 数をまたいで暖機起動しない。各 Mach 数の :math:`\alpha=\beta=0` は自由流から始め（冷えた起動）、迎角と横滑り角の列だけを暖機起動でつなぐ。
残差の大きさは Mach 数で変わるので、受け入れの基準にする残差の最大値（:ref:`sec-cfd-int-rans-accept`）を Mach 数ごとに冷えた起動で決めるためである。
SA の変数を持たない解（Euler の解）からは始めない。暖機起動のケースが失敗したときは、自由流から解き直す。
この依存関係のもとで、準備のできたケースを ``parallel_cases`` 個まで同時に実行する。
最初に :math:`\alpha=\beta=0` の列が Mach 数順に解かれ、そのあと各 Mach 数の角度の列が並列に進む。

各ケースは ``mpirun -n <ranks> --oversubscribe --bind-to none SU2_CFD case.cfg`` で実行する（ランク数が 1 のときは ``SU2_CFD case.cfg`` を直接）。
単一ノードの実行になるので、MPI の通信は共有メモリだけにしている。
Ctrl-C や SIGTERM を受けると、子プロセスをまとめて終了し、終わったケースの結果は残す。


.. _sec-cfd-int-convergence:

再開と収束の判定
~~~~~~~~~~~~~~~~

各ケースの出力、再開の規則、収束の判定を述べる。

各ケースは ``<出力先>/<workdir>/m0.800_a04.00/`` のようなディレクトリに入る（負の迎角は ``a-04.00``）。
横滑り角があれば ``_b04.00``、ロール減衰のケースは ``_r+0.0100`` と ``_r-0.0100`` を名前に足す（例：``m0.800_a04.00_b04.00``）。
SU2 の設定、ログ、履歴（``history.csv``）、壁面の分布（``surface_flow.csv``）があり、終わると ``done.json``\ （入力のハッシュ、係数、収束の状態。項目は :ref:`sec-file-cfd`）が書かれる。
RANS の冷えた起動では、1 次精度の段の設定、ログ、履歴（``case_startup.cfg``、``su2_startup.log``、``history_startup.csv``）も残る。
入力のハッシュは SU2 の設定（暖機起動の行を除く）とメッシュのハッシュから作る。
同じハッシュの ``done.json`` があるケースは次の実行で飛ばす。途中で止めても、あるいは設定を一部変えても、影響を受けないケースは再利用される。
最初からやり直すには、このディレクトリを削除する。

Euler のケースの結果は、次のいずれかで扱う（RANS は :ref:`sec-cfd-int-rans-accept`）。

* **収束**\ ：残差の目標（``convergence``\ 、既定 :math:`10^{-6}`）に達し、係数の変化が落ち着いた。最後の反復の係数を使う。
  SU2 は目標より 2 桁以上低い残差（:math:`10^{-14}` まで）を止める値にして走らせ、ランナーが次の二つを満たした時点でケースを止める。
  ケースを止めるときは、SU2 の次の定期的な出力（暖機起動に使う ``restart.dat``）を待ってから止める。
  直近 ``stable_window`` 反復の力（CFx、CFz。横滑りのケースは CFy も）の最大と最小の差が力の大きさの 0.2 %（＋1e-5）以内で、圧力中心の差が全長の 0.002 倍以内であること。
  さらに、その窓の係数の傾きに、残差の減り方から求めた時定数 :math:`\tau=1/(r\ln10)`\ （:math:`r` は窓での残差の常用対数の減る速さ、上限は ``iterations``）をかけた、これから先の変化の見込みが同じ許容以内であること。
  残差が目標に達しても係数がゆっくり振動していることがあり（サンプル機体の :math:`M=0.8` で、残差が :math:`10^{-6}` に達した時点の :math:`C_N` は収束した値より 0.56 % 低かった）、残差だけでは止めない。
  反復の上限に達しても係数が落ち着かないときは、残差が目標に達していれば窓の平均を係数として受け入れ、「収束」には数えない（``converged`` は false で ``cfd_report.json`` の ``unconverged`` に載る）。
  そのときは ``warning[IY-F5nn]: aero.cfd: case m0.800_a04.00: the coefficients still trend or oscillate at the end (… iterations): their range over the last stable_window iterations or their change extrapolated from the residual decay exceeds 0.2 % of the force scale`` の形の警告を出す。
* **早期終了**\ ：``min_iterations``\ （既定 100）反復を過ぎて、残差が最大値から 3 桁以上下がり、直近 ``stable_window``\ （既定 60）反復の力とモーメントが落ち着き（前半と後半の平均の差が力の大きさの 2 % 未満など）、上の収束と同じ係数の変化の判定も満たしたとき、ケースを止めて受け入れる（``status`` は ``accepted``）。
  落ち着きの判定は、どの場合も軸力 CFx、法線力 CFz、ピッチングモーメント CMy（許容は力の大きさの 2 %＋5e-4、CMy は 25 %＋5e-3）で行い、ケースの種類に応じて次を加える。
  横滑りのケース（:math:`\beta\ne0`）は横力 CFy（力と同じ許容）とヨーイングモーメント CMz（CMy と同じ許容）、ロール減衰のケースは転がりモーメント CMx（自身の平均の 3 %＋2e-5。転がりモーメントは力に比べてはるかに小さいので、自分の大きさで測る）である。
  力だけの判定では、CMx が窓の中でまだ変化していても受け入れてしまうからである。
  ``min_iterations`` は ``stable_window`` 以上でなければならない。Euler の Roe の制限関数は ``min_iterations`` 反復で固定する（``LIMITER_ITER``）。既定値の根拠の測定は :ref:`早期終了の測定 <sec-cfd-early-stop>` に示す。
* **受け入れ**\ ：反復の上限か時間の上限（``timeout_minutes``、既定 0 で無制限）に達したが、残差が最大値から 3 桁以上下がり、
  最後の窓（反復数の 1/4、2 から 100 反復）で早期終了と同じ落ち着きの判定を満たすとき、係数を窓の平均として受け入れる。どちらの場合も「収束」には数えず、``cfd_report.json`` の ``unconverged`` に載る。
  残差が目標に達していても、その時点で係数がまだ傾いているか振動している（上限に達した）ときも、同じ窓の平均を使い、``the coefficients still trend or oscillate at the end (… iterations): … coefficients averaged over the last N iterations`` を警告に記録して未収束とする（:math:`N` は窓の反復数）。
  早期終了で止めたケースは、止めると決めた時点までの履歴と判定で扱う。止めてから SU2 が次の書き出しまで進めた行は、判定を変えない（RANS も同じ）。
  残差は下がりきらなくても係数が平らになる状況（例えば底面の後流が振動する場合）を、結果として使うための扱いである。
* **失敗**\ ：SU2 のエラー、発散、履歴なし、あるいは収束も安定もしていない場合。``done.json`` は作らない。
  表の作成では、失敗したケースを飛ばして補間する（:ref:`sec-cfd-post`）。

.. _sec-cfd-int-rans-stages:

RANS の二段の計算と発散の扱い
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

RANS のケースを自由流から始めるとき（冷えた起動。各 Mach 数の :math:`\alpha=\beta=0` と、暖機起動に失敗したケース）は、同じディレクトリで二段に分けて解く。

1. **1 次精度の段**\ ：再構成のない Roe で、自由流から解く。反復は ``iterations`` と 500 の小さいほうまでで、100 反復目からは、密度の残差がその最大値から 2 桁下がった時点で終える（``REL_RMS_DENSITY``）。
   解は ``restart_startup.dat`` に書く。この段の密度の残差と SA の残差の最大値が、そのケースと、そこから暖機起動でつながるケースの受け入れの基準（基準の最大値）になる。
2. **2 次精度の段**\ ：``restart_startup.dat`` から始め、MUSCL の再構成で解く（:ref:`sec-cfd-int-discretization`）。反復数の上限は ``iterations`` である。

暖機起動のケースは、隣のケースの ``restart.dat`` から 2 次精度の段だけを解き、基準の最大値を隣のケースから受け継ぐ（``done.json`` の ``reference_peak``）。

ランナーは 2 次精度の段の履歴を 1 秒ごとに読み、発散を見つけるとすぐに SU2 を止める。発散は、最後の 5 行の中央値で次のどれかが起きたときとする。

* 密度の残差が、10 行目以降の最小値より 2 桁以上大きい。SA の残差なら 3 桁以上大きい。
* 残差か係数が有限でない。
* 力の係数の大きさが 100 を超える。

1 次精度の段が発散したとき（SU2 が残差 1e20 超で止まった場合を含む）は、その段を一度だけ、適応 CFL 数の上限を :math:`4\,\mathrm{cfl}` に下げて（通常は :math:`40\,\mathrm{cfl}`）解き直す。SA の方程式の CFL 数は流れの 0.2 倍にする（通常は 1 倍）。
このとき ``the first-order startup diverged (…); repeated once with the adaptive CFL capped at 20 instead of 200 (the SA equation stays at 0.2 of the flow CFL); the second-order stage keeps these settings``\ （``cfl`` = 5 のとき）を警告として記録し、2 次精度の段にもこの設定を使う。
解き直しも発散すれば、``the first-order startup diverged twice (the second time with conservative settings): …`` で失敗とする。

冷えた起動のケースの 2 次精度の段が発散したときは、``restart_startup.dat`` から、同じ保守的な設定（適応 CFL 数の上限 :math:`4\,\mathrm{cfl}`、SA の CFL 数は流れの 0.2 倍）で 2 次精度の段を一度だけ解き直す（制限関数は固定しない）。
そのとき ``the second-order stage diverged (…); repeated once from the first-order solution with the adaptive CFL capped at 20 instead of 200 and the SA equation at 0.2 of the flow CFL instead of 1``\ （``cfl`` = 5 のとき）を警告として記録する。
解き直しも発散すれば、``diverged twice (the second time with conservative settings): …`` で失敗とする。
解き直した 1 次精度の段のあとで発散したときは、``the second-order stage diverged with the conservative settings of the repeated first-order startup: …`` で失敗とする。
暖機起動のケースが発散したとき、シグナルによらずに SU2 が異常終了したとき、反復の上限で受け入れの判定を満たさなかったときは、``the warm start failed (…); solved again from the freestream`` を記録して、冷えた起動で解き直す。

.. _sec-cfd-int-rans-accept:

RANS の受け入れの判定
~~~~~~~~~~~~~~~~~~~~~

RANS のケースは、2 次精度の段の履歴を、次の量で判定する。

* **基準の最大値**\ ：冷えた起動の 1 次精度の段の、密度の残差と SA の残差の最大値（:ref:`sec-cfd-int-rans-stages`）。2 次精度の段の残差がそれより大きくなったときは、その段の最大値を使う。
  暖機起動のケースの残差は低い値から始まるので、自分の最大値から測ると早く受け入れすぎるためである。
* **残差の下がり**\ ：基準の最大値と、最後の 20 行の残差の最大値の差（桁）。振動する残差が一時的に下がった値では受け入れない。
* **係数の平らさ**\ ：直近 ``stable_window``\ （RANS の既定 200）行で、軸力 CFx と法線力 CFz（横滑りのケースは横力 CFy も）の前半と後半の平均の差と四分位範囲が、力の大きさ（平均の絶対値の最大）の 0.5 %（:math:`+10^{-5}`）以下であること。
  圧力中心 :math:`-C_{My}/C_{Fz}`\ （横滑りのケースは :math:`C_{Mz}/C_{Fy}` も）の差と四分位範囲は、全長の 0.002 倍以下とする。法線力がほぼ 0 で圧力中心が定まらないときは、モーメントそのものに力の許容に全長をかけた許容を使う。
  ロール減衰のケースは RANS では解かない。
* **傾きの外挿**\ ：窓の中の係数の傾き（1 行あたりの最小二乗の傾き）に、残差の減り方から決める時定数 :math:`\tau` をかけた値を、これから残る変化とみなし、平らさと同じ許容以下であることを求める。
  係数が残差とともに :math:`\exp(-i/\tau)` のように収束すると、残る変化は傾きの大きさの :math:`\tau` 倍である。
  :math:`\tau=1/(r\ln10)` で、:math:`r` は窓の中の密度と SA の残差の減る速さ（1 反復あたりの桁）の遅いほうである。:math:`\tau` は ``iterations`` を上限とし、残差が減っていなければ ``iterations`` とする。

判定の結果は次の 4 つである（上から順に調べる）。

* **収束**\ （``converged``）：密度の残差が 3 桁以上、SA の残差が 2 桁以上下がり、係数が平らで、傾きの外挿も許容以下。最後の行の係数を使う。
* **停滞**\ （``unconverged``）：直近の窓の密度の残差の中央値が基準の最大値より 0.5 桁以上下にあり、その前の窓の中央値から 0.2 桁以上は下がっておらず、二つの窓を合わせた区間で係数が平らで傾きの外挿も許容以下。
  係数は二つの窓の平均とし、``unconverged: residual stalled … orders below the reference (median of the last … iterations), coefficients stationary over … iterations: CFx …, CFz …`` の形で、残差の位置と力の平均と四分位範囲を警告に記録する。
* **振動**\ （``unconverged``）：密度の残差が 2 桁以上下がり、窓の中の力の四分位範囲が力の大きさの 1 % 以下、圧力中心の四分位範囲が全長の 0.004 倍以下（平均の差は問わない）。係数は窓の平均とし、``limit cycle: coefficients averaged over the last … iterations`` を警告に記録する。
* **失敗**\ ：どれでもない。``not converged (iteration limit): density residual … and SA residual … orders below the cold reference peak (need 3 and 2), coefficients …`` の形で、残差の下がりと係数の状態（``stable``、``flat but still trending``、``in a limit cycle``、``still changing``）を示す。

ランナーは ``min_iterations``\ （RANS の既定 400）行を過ぎてから 1 秒ごとに判定し、収束か停滞になったら止める。
SU2 は SIGTERM で止めると何も書かないので、すぐには止めず、``restart.dat`` と ``surface_flow.vtk`` が次に書き出され（50 反復ごと）、さらに履歴が 1 行進むのを待ってから止める（最大 105 行）。
暖機起動に使う解と、:math:`y^+` を確かめる壁面の分布が、受け入れた解のものになる。
振動は、反復の上限か時間の上限で SU2 が終わったときに調べる。

傾きの外挿は、平らさだけでは遅い変化を見逃すために加えた条件である。
NASA TMR の平板（格子 69×49、137×97、273×193、:math:`M=0.2` と 2）の SU2 の履歴にこの規則を当てはめると、平らさと残差の条件だけでは、:math:`M=0.2` の 273×193 で 1517 反復目に止まり、
そこでの平板の抗力は残差を 8 桁下げた値と 1.9 % 違う。傾きの外挿を加えると 2465 反復目に止まり、差は 0.08 % になる。6 つの格子と Mach 数の組で、差は最大 0.3 % である（``doc/data/cfd/rans_flatplate.csv``）。

受け入れたケースでは、``surface_flow.vtk`` の壁面の :math:`y^+`\ （底面の節点を除く）の 95 パーセンタイルを ``done.json`` の ``yplus_p95`` に記録する。
``yplus`` が 5 以下（壁面を解像する設定）でこの値が 3 を超えるときは、``wall y+ P95 = … exceeds 3 (target yplus = …): the first cell is too thick for the resolved wall`` を警告にする。
:math:`y^+` が分布に含まれなければ ``wall y+ not checked: no Y_Plus in surface_flow.vtk`` を警告にする。
ケースの警告は ``warning[IY-F5nn]: aero.cfd: case m0.800_a04.00: …`` の形で標準エラー出力にも表示する。
``done.json`` の ``converged`` は、収束で真、停滞と振動で偽になり、偽のケースは ``cfd_report.json`` の ``unconverged`` に載る。
停滞のケースは ``done.json`` の ``stalled`` が真になり、判定の根拠にした密度の残差の下がり（停滞では直近の窓の中央値）を ``residual_drop`` に記録する。
``cfd_cases.csv`` には、基準の最大値の密度の値（``residual_reference``）、この下がり（``residual_drop``）、判定（``rans_verdict``：``converged``、``limit_cycle``、``stalled``）が入る。Euler では空である。
``residual_first`` と ``residual_last`` は 2 次精度の段の履歴の最初と最後の行の密度の残差である。
``cfd_report.json`` の ``stalled_cases`` に停滞のケースと下がりを挙げ、実行の最後に次の警告を出す。

.. code-block:: text

   N RANS case(s) accepted with a stalled residual, not converged (coefficients averaged over two windows; a stalled residual needs only 0.5 orders below the reference, convergence 3): <ケース> (<下がり> orders below the reference), …; check them in cfd_cases.csv (rans_verdict, residual_drop)


.. _sec-cfd-post:

後処理
------

本節は、SU2 が出した力とモーメントの係数から、飛翔計算が読む係数表を作る手順を述べる。
座標と符号の変換、CFD が与えない項の補い方、解いた点から規則的な表の格子への補間、一般表の作り方、ロール減衰微係数の取り出しの順に進む。

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
半領域の解では、:math:`y=0` の対称面が半分の機体の圧力を受けるので、:math:`C_Y,C_l,C_n` は対称面による見かけの値である。機体は :math:`y=0` について対称なので、半領域のケースではこれらを 0 にする。
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
  ただし ``surface = "stl"`` と ``"step"`` では、フィンの前縁と後縁の面が CFD の壁面にあり、その圧力抗力は :math:`C_A^{\mathrm{CFD}}` にすでに入っているので、フィン縁の項 :math:`C_{D,\mathrm{fin}}` を足さない（足すと二重に数える）。
  ``panel_mesh`` の形状は、抽出した厚みのフィンを再構成するので、この項を足す。
  ``surface = "stl"`` と ``"step"`` では、フィンの後縁が底面にそろっていると、後縁の面は ``base`` のマーカーに入って力に積分されないので（上の底面の規則）、その底面抗力を次の項で足す。
  後縁がそろっているとは、根元と翼端の後縁がどちらも底面の平面 :math:`x=L` から :math:`10^{-4}L` 以内にあることをいう。

  .. math::
     :label: eq-cfd-te-flush

     C_{D,\mathrm{TE,flush}}=\frac{C_{D,\mathrm{base}}(M)\,n\,b\,t}{S_{\mathrm{ref}}}

  ここで :math:`C_{D,\mathrm{base}}(M)` は部品積み上げ法の底面抗力係数、:math:`n` はフィンの枚数、:math:`b` は span、:math:`t` は抽出した平均のフィンの厚さである。
  この項は :eq:`eq-cfd-ca` の和に入れ、``fin_te_square = false`` のとき、後縁が底面から内側にある（後縁の面が ``wall`` にある）とき、``panel_mesh`` のときは 0 である（``panel_mesh`` ではフィン縁の項全体 :math:`C_{D,\mathrm{fin}}` を足す）。
  平均の厚さを使うので、後縁が鈍いくさび形のとき、実際の底面抗力より小さい。
  燃焼中の値では、底面の項に、ノズル出口の面積を除いた底面積を使う。
* **RANS**\ ：SU2 の履歴のマーカごとの列（``CFx(wall)``、``CFx(base)``）を使う。壁面の軸力 :math:`C_{Fx}(\mathrm{wall})` は、圧力と摩擦、フィンの縁とボートテールの力を含む。
  底面の項は ``rans_base`` で選び、

  .. math::
     :label: eq-cfd-ca-rans

     C_A=\max\Bigl(C_{Fx}(\mathrm{wall})+C_{A,\mathrm{base}}+f(\alpha)\bigl(C_{D,\mathrm{extra}}+\Delta C_{D,\mathrm{rough}}+\Delta C_{D,\mathrm{fin}}\bigr),\,0\Bigr),\qquad
     C_{A,\mathrm{base}}=\begin{cases}f(\alpha)\,C_{D,\mathrm{base}} & (\text{rans\_base = model})\\ C_{Fx}(\mathrm{base}) & (\text{rans\_base = cfd})\end{cases}

  とする。``"model"``\ （既定）は部品積み上げ法の底面抗力（燃焼中はノズル出口の面積を除いた底面積）、``"cfd"`` は RANS が平らな底面に積分した力である。
  ``"model"`` で ``surface = "stl"`` と ``"step"`` のとき、フィンの後縁がそろっていれば、:eq:`eq-cfd-te-flush` の :math:`C_{D,\mathrm{TE,flush}}` を :math:`f(\alpha)` をかけて足す。``"cfd"`` では足さない（後縁の面は ``base`` のマーカーの力に入っている）。
  ``"cfd"`` の燃焼中の値は、ノズル出口の面積にかかる底面圧力の寄与（部品積み上げ法の燃焼後と燃焼中の底面抗力の差）を引く。
  RANS の壁は滑らかなので、設定の粗さの効果を、部品積み上げ法の粗い壁と滑らかな壁の摩擦の差として足す。
  :math:`C_{D,f}` は粗さ :math:`k_s` の摩擦（:eq:`eq-cd0`）、:math:`c_f` は摩擦係数の式で、同じ Reynolds 数での比を使い、

  .. math::
     :label: eq-cfd-rans-rough

     \Delta C_{D,\mathrm{rough}}=C_{D,f}\Bigl(1-\frac{c_f(Re,M,0)}{c_f(Re,M,k_s/L)}\Bigr)

  とする（粗さ 0 なら 0）。
  作り直した形状（``panel_mesh``）では、フィンの前縁と後縁を弦長の割合 :math:`e=0.02` で鈍くしてあり（:ref:`sec-cfd-rebuild`）、その縁の面の厚さは最大の厚さの :math:`4e(1-e)=7.8` % である。
  設定の縁（``fin_le``\ 、``fin_te``）の抗力は縁の厚さに比例するとして、部品積み上げ法のフィン縁の項 :math:`C_{D,\mathrm{fin}}` から、鈍くした縁がすでに壁面の力に持つ分を引いて

  .. math::
     :label: eq-cfd-rans-fin

     \Delta C_{D,\mathrm{fin}}=C_{D,\mathrm{fin}}\bigl(1-4e(1-e)\bigr)

  を足す。``surface = "stl"`` と ``"step"`` では縁の面が CFD の壁面にあるので、:math:`\Delta C_{D,\mathrm{fin}}=0` である。
  どちらの増分もモデルによる補正で、CFD の解ではない。部品積み上げ法の摩擦とボートテールの項と、Euler の項は足さない。
  履歴にマーカごとの列がないときは、監視する力の合計（``wall`` と ``base``\ 。底面を含む）を使い、表全体を ``rans_base = "cfd"`` と同じに作って、``cfd_report.json`` の ``warnings`` に記録する。
  ケースごとの :math:`C_{Fx}(\mathrm{wall})`、:math:`C_{Fx}(\mathrm{base})`、部品積み上げ法の摩擦と底面の値、増分（粗さ、フィン縁、そろった後縁の底面 ``fin_te_base_increment``）は、``cfd_report.json`` の ``axial_breakdown`` に並べて記録する（部品積み上げ法の摩擦は比べるための値で、表に足さない）。

亜音速の Euler では、CFD の軸力の理論値は 0（d'Alembert のパラドックス）である。実際の値は小さな負の数（サンプル機体の迎角 0〜8° で :math:`-0.01` から :math:`-0.05`）になり、そのまま合計に入る。

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

とする。これらの和から :math:`C_{mq}`、:math:`C_{nr}` が決まる関係は :ref:`sec-aero-damping` にある。CFD が直接与えるのは傾きと重心位置だけである。

.. _sec-cfd-table-build:

係数表の作成
~~~~~~~~~~~~

解いた点は不規則な格子（解いた Mach 数と迎角）なので、それを係数表の規則的な格子に埋める。

1. 各 Mach 数の列で、:math:`C_N` とノーズ周りのモーメント :math:`m` は迎角について奇関数とし、原点 :math:`(0,0)` を通る。
   :math:`C_N,m` と CFD の軸力を、迎角について単調な 3 次補間（PCHIP :cite:`fritsch`）する。
   解いた最大の迎角 :math:`\alpha_e` より大きい表の行は、部品積み上げ法へつなぐ（下記）。端の傾きは延長しない。
2. :math:`C_{N\alpha}` と :math:`x_{cp}(\alpha\to0)` は、最小の二つの正の迎角を通る奇関数の 3 次式 :math:`c_1\alpha+c_3\alpha^3` から求める（当てはめた傾き :math:`c_1` が最小の迎角での割線の傾きから 5 割を超えてずれるときは、割線の傾きを使う）。
   :math:`x_{cp0}` は、モーメント :math:`m` に同じ当てはめをした傾きを :math:`C_{N\alpha}` で割った値である。
   手順 1 の :math:`C_N` と :math:`m` の PCHIP は、原点での傾きを、この当てはめの傾き（:math:`C_N` では :math:`C_{N\alpha}`）にして始める。
   この傾きは最小の迎角での割線の傾きの 0.5〜1.5 倍（または割線の傾きそのもの）なので、PCHIP の単調性の条件（割線と同じ符号で 3 倍以内）を満たす。
   こうすると、表の :math:`C_{N\alpha}`、迎角 0 の圧力中心 :math:`x_{cp0}`、減衰の和（:eq:`eq-cfd-damp`）と、表の :math:`C_N` と :math:`m` の行が、一つの曲線から求めたものになる。
3. Mach 数の方向にも、各量を係数表の Mach 数の格子に PCHIP で補間する。解いた Mach 数の範囲の外には外挿せず、最小または最大の Mach 数の値で一定とする。
   迎角 0 の圧力中心は、:math:`x_{cp0}` そのものではなく、モーメントの傾き :math:`C_{N\alpha}x_{cp0}` と :math:`C_{N\alpha}` をそれぞれ補間した比 :math:`(C_{N\alpha}x_{cp0})/C_{N\alpha}` とする（:math:`|C_{N\alpha}|\le10^{-9}` では :math:`x_{cp0}` を補間する）。
   0 でない迎角の行が、補間したモーメントを補間した :math:`C_N` で割った圧力中心を持つ（:eq:`eq-xcp-interp`）のと同じ扱いで、解いた Mach 数の間でも、迎角 0 の圧力中心と減衰の和（:eq:`eq-cfd-damp`）が小さい迎角の行と食い違わない。
   途中の Mach 数の列が失敗していれば、両隣の列で補間して埋める。最小または最大の Mach 数の列の失敗は補えず、エラーになる。
4. 軸力に上の項を足し、ピッチ減衰の和を取り替える。``roll_damping`` が真のときは、:math:`C_{lp}` も CFD の値（:ref:`sec-cfd-roll-post`）にする。

解いた迎角より大きい範囲には CFD のデータがない。端の傾きを延長した直線は、最後の区間で圧力中心が動く向きに圧力中心を際限なく動かし、大きな迎角で重心より前に出すことがある。
そうなると大きな迎角に見かけの安定な釣り合い（トリム）ができ、頂点のあとの弾道落下が、遅く遠くまで流される落下になる。
そこで、Mach 数の列ごとに、:math:`\alpha_e` での CFD の値（:math:`C_{N,e}`、:math:`x_{cp,e}=m_e/C_{N,e}`、:math:`C_{A,e}`）から部品積み上げ法の値（添字 B）へ移す。
部品積み上げ法のフィンの法線力は大迎角で有界で（:ref:`sec-fin-large-alpha`）、フィンの圧力中心は平面形の図心へ動く。

.. math::
   :label: eq-cfd-beyond

   C_N(\alpha) &= C_{N,B}(\alpha)\,\bigl(1+(r-1)\,w(\alpha)\bigr),\qquad r=\frac{C_{N,e}}{C_{N,B}(\alpha_e)}\\
   x_{cp}(\alpha) &= x_{cp,B}(\alpha)+\bigl(x_{cp,e}-x_{cp,B}(\alpha_e)\bigr)\,w(\alpha)\\
   C_A(\alpha) &= C_{A,e}\,\frac{f(\alpha)}{f(\alpha_e)}

ここで :math:`f` は部品積み上げ法の軸力の迎角の倍率（:eq:`eq-ca-alpha`）、:math:`w` は :math:`\alpha_e` で 1、:math:`\alpha_r=\min(2\alpha_e,\,90^\circ)` で 0 になる、両端の傾きが 0 の 3 次 Hermite 補間で、:math:`\alpha\ge\alpha_r` では 0 である。
:math:`r` が正でないとき（:math:`C_{N,e}` と :math:`C_{N,B}(\alpha_e)` の符号が違う）は、比の代わりに差を使い :math:`C_N=C_{N,B}(\alpha)+(C_{N,e}-C_{N,B}(\alpha_e))\,w(\alpha)` とする。
:math:`x_{cp}` は機体の内側（0 から全長）に制限する。
値は :math:`\alpha_e` で CFD の値に一致し、:math:`\alpha_r` から先は部品積み上げ法の法線力と圧力中心になる（軸力は CFD の水準に部品積み上げ法の形を掛けたもので、:math:`90^\circ` で 0）。
表は節点の間を線形に補間するので、値の一致が表として意味のある連続性である。
このつなぎは ``extrapolation`` の設定によらない。``extrapolation`` は表に記録され、表の格子の外の引き出しだけに効く（:ref:`sec-aero-interp`）。
``examples/sample_cfd.toml``\ （解く迎角 0、4、8°、``alpha_max_deg = 90``）では、8° から 16° の間でつなぎ、16° から 90° は部品積み上げ法の値になる。

係数表の入力ハッシュ（キャッシュキー）には ``[aero.cfd]`` の設定が含まれるが、結果を変えない資源とパスの項目（``prefix`` ``su2`` ``mpi`` ``ranks_per_case`` ``parallel_cases`` ``timeout_minutes`` ``workdir``）は含まれない。
ハッシュには、プログラムの版と、空力モデルの版（係数の計算方法が変わると上げる番号）も入る。
個々のケースの結果（``done.json``）は SU2 の設定とメッシュのハッシュで判定するので、モデルの版が変わっても SU2 は再実行せず、保存したケースから表だけを作り直す。


.. _sec-cfd-general:

一般表の作成
~~~~~~~~~~~~

一般表（6 成分、:ref:`sec-aero-coeff6`）になる条件は :ref:`sec-cfd-table-kind` にある。本節はその作り方を述べる。

一般表の格子は、迎角が :math:`-180^\circ` から :math:`180^\circ`、横滑り角が :math:`-90^\circ` から :math:`90^\circ` である。
どちらも非一様で、:math:`\pm30^\circ` までは 2 度刻み、その外側は 10 度刻み（迎角 61 点、横滑り角 43 点）とする。
Mach 数の刻みは ``general_mach_step``\ （既定 0.1、0 なら ``[aero]`` の ``mach_step``）で、範囲は ``[aero]`` の ``mach_min`` から ``mach_max`` である。
格子は 1 つの Mach 数あたり :math:`61\times43` 点で、Mach 数の刻みを粗くするとファイルが小さくなる（ファイルの形式は :ref:`sec-file-general`）。

Mach 数ごとの作り方は次のとおりである。

1. **対称性による補完**\ 。解いた点に、形状の鏡映（:ref:`sec-cfd-body6`）の像を加える。``alpha_mode = "mirror"`` では、同じ :math:`(\alpha,\beta)` に重なるすべての像を平均し、数値的な非対称を消す。
   そうでなければ、解いた点が像より優先される。``"mirror"`` と ``"offset"`` では、:math:`(0,0)` の点の :math:`C_Y,C_Z,C_m,C_n` を、先に全点から引く。
   :math:`C_l` は鏡映対称な形状でだけ引き、鏡映対称でない形状ではカントしたフィンのロールの強制力として残す。
   鏡映対称な形状ではこれらは厳密に 0 で、``"mirror"`` では像との平均でも消える。像のない成分（鏡映対称でない形状のすべて、``"pitch_plane"`` の :math:`C_Z,C_m`）は、この引き算で取り除かれる（:ref:`sec-cfd-val-asym-cause`）。
   そのあとで、形状の対称性から厳密に 0 になる成分を 0 にする。:math:`y=0` について鏡映対称な形状（``"axisymmetric"`` と ``"pitch_plane"``）では、:math:`\beta=0` の点の :math:`C_Y,C_l,C_n` を 0 にし、
   :math:`z=0` についても対称な形状（``"axisymmetric"``）では、さらに :math:`\alpha=0` の点の :math:`C_Z,C_l,C_m` を 0 にする。
   半領域の対称面での積分に残る数値的な残りを取り除くためで、``"none"`` では解の値をそのまま使う。
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
5. **Mach 数の方向**\ 。格子の点ごとに、解いた Mach 数の値を PCHIP で補間する。解いた範囲の外は外挿せず、端の値で一定とする（``extrapolation`` にかかわらない）。
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
Mach 数の方向には、軸対称表と同じく :math:`C_{N\alpha}` とモーメントの傾き :math:`C_{N\alpha}x_{cp0}` を PCHIP で補間し、減衰の重心 :math:`x_{cp0}` をその比とする（:math:`|C_{N\alpha}|\le10^{-9}` では :math:`x_{cp0}` を補間する）。
横滑りを解かないときは、ヨーの和をピッチの和に等しくし、``cfd_report.json`` の ``yaw_damping_from_pitch`` が真になる。
:math:`C_{N\alpha}` と :math:`x_{cp0}` は一般表の入力ではなく、``cfd_report.json`` の ``diagnostics`` に記録する診断値である。
ピッチとヨーを別々に求めるので、機体が非対称なときは異なる値になる。

ロールのカント強制 :math:`C_{l\delta}` は、部品積み上げ法の値を使う。
一般表は、表の :math:`C_l` に含まれるカント :math:`\delta_{\mathrm{ref}}` を記録する（:eq:`eq-cl-total`）。
STL モードでは、形状（STL）にカントが含まれているので、:math:`\delta_{\mathrm{ref}}` は設定したフィンのカントである。
カントの影響は :math:`C_l` にすでに入っており、:math:`\delta=\delta_{\mathrm{ref}}` の飛翔では強制項を足さない（二重に数えない）。
``panel_mesh`` は形状にカントがないので、:math:`\delta_{\mathrm{ref}}=0` である。


.. _sec-cfd-region-outside:

CFD の領域の外の扱い
~~~~~~~~~~~~~~~~~~~~

一般表の値のうち、解いた迎角と横滑り角の範囲の外側は、CFD の結果ではなくモデルの値である。
本節は、どこまでが CFD の値で、どこからがモデルの値かを、実装がどう決めて記録するかを述べる。

解いた範囲 :math:`R` の内側は CFD の補間値である。外側は、部品積み上げ法の軸対称表を 6 成分に展開した値の変化を、:math:`R` の境界の CFD の値に合わせて足したもので、:eq:`eq-cfd-fill` により境界をまたいで連続である。
一般表を作ったときは、``cfd_report.json`` の ``cfd_region`` に :math:`R`\ （迎角と横滑り角の最小・最大）を、``model_outside_cfd_region`` に真を記録する。
係数表のファイルには領域の情報は付かないので、どこまでが CFD の値かは、このレポートで確かめる。
展開は軸対称な形状のものなので、非対称な形状の :math:`R` の外側では当てにならない。

Mach 数の方向は異なり、解いた Mach 数の範囲の外は、``extrapolation`` にかかわらず、端の値で一定とする。
表の Mach 数の格子が解いた範囲より広いと、``cfd_report.json`` に ``mach_clamped_above``\ （最大の解いた Mach 数。それより上で一定）または ``mach_clamped_below``\ （最小の解いた Mach 数。それより下で一定）を記録し、同じ内容の警告を ``warnings`` に加える。
軸対称表では、解いた最大の迎角より大きい行を部品積み上げ法へのつなぎ（:eq:`eq-cfd-beyond`）で埋める。
CFD が与えない項（摩擦、底面、フィン縁、ボートテールの抗力）とピッチ・ヨー減衰の和は、部品積み上げ法（またはパネル法）のモデルから補う（:ref:`sec-cfd-post`）。

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

表の値は、部品積み上げ法の :math:`C_{lp}(M)` に、使える Mach 数での CFD / 部品積み上げ法の比を PCHIP で補間して（範囲外は端の値で一定）かけた値である
（部品積み上げ法の値が 0 に近いときは、CFD の値そのものを補間する）。
使える Mach 数がなければ部品積み上げ法の値になる。``clp_source`` は ``"cfd"``\ （すべての Mach 数が CFD）、``"barrowman"``\ （解かない、またはすべて不採用）、``"mixed"`` のいずれかである。
ロール減衰は Euler の解であり、粘性の寄与（フィンの摩擦）は含まない。

確認済み
--------

CFD の実装について、次の点を確かめた。数値と手順は :ref:`sec-vaero-cfd` に示す。

* 円すいの表面圧力は、メッシュを細かくすると Taylor–Maccoll の理論解に近づく。
* 迎角の正負を解いて奇関数と偶関数に分ける処理（``alpha_mode = "mirror"``）と :math:`z` 対称メッシュで、:math:`\alpha=0` の見かけの非対称性が消える。
* 回転座標系のロール減衰の符号と無次元化、6 成分への写像と ``SIDESLIP_ANGLE`` の符号は、軸対称表を展開した値と一致する。
* 亜音速の :math:`C_{N\alpha}` は、CFD とパネル法の差が 2.5 %（:math:`M=0.8`）から 9 %（:math:`M=0.3`）である。手法どうしの相互比較であり、実測との比較（妥当性確認）ではない。
* STL をそのまま解く計算は、作り直した形状による計算や Barrowman 法と相互比較した。

注意
----

実装上の制約は :ref:`sec-lim-cfd` にまとめた。本章に関係する主なものは次のとおりである。

* RANS のメッシュは ``panel_mesh`` の形状で、フィンの配置が :math:`y=0` について対称なときだけ作れる。RANS の検証の項目と結果は :ref:`sec-cfd-rans` にある。
* メッシュ収束は完全でなく、STL のメッシュは作成のたびに少し変わる。
* Euler の底面は尾部フェアリングで避けるので、底面抗力は経験式による。
* ピッチ・ヨー減衰（:math:`C_{mq}`、:math:`C_{nr}`）は CFD が直接与えるものではなく、交差する微係数は扱わない。
* 一般表の CFD の領域の外はモデルの値である。
* 形状の忠実度の距離はサンプリングによる近似で、係数表のハッシュに CFD の実装の版は含まれない。
