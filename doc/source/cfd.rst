CFD を使う
==========

本章は、CFD（計算流体力学）で係数表を作る方法を、使う側の視点で説明する。
部品積み上げ法（:doc:`barrowman`）とパネル法（:doc:`panel`）は、機体のまわりの流れ場を解かずに係数を求める。
CFD は、機体のまわりの空間を格子で覆い、圧縮性流れの方程式を数値的に解く。
設定ファイルで ``aero.method = "cfd"`` を選ぶと、他の手法と同じ形式の係数表（:ref:`sec-aero-table`）が CFD の結果から作られ、飛翔計算は何も変えずにそれを使う。
方程式、数値解法、メッシュの作り方、後処理の詳細は :doc:`cfd_internals` にある。

CFD でできること
----------------

いつ CFD を使うか
~~~~~~~~~~~~~~~~~

三つの手法の違いと計算量は :ref:`sec-aero-methods`、手法の選び方は :ref:`sec-aero-select` にまとめた。

CFD は、衝撃波や遷音速の流れ、フィンと胴体の干渉を物理方程式から直接得られる。
STL をそのまま解くこともでき、横滑りとロールを含む 6 成分の係数表（:ref:`sec-aero-coeff6`）を作れる。
一方で、1 ケースごとに 10\ :sup:`5` から 10\ :sup:`6` 個の格子点の流れ場を求めるため、1 ケースが数分、行列全体では数十分から数時間かかり、メモリも使う（:ref:`sec-cfd-run` に目安を示す）。
そこで使いどころは、部品積み上げ法やパネル法の結果を独立な方法で確かめたいとき、あるいは遷音速・超音速の係数をより信頼できる値にしたいときである。
CFD が出す係数表は、他の手法の表と同じ形式なので、飛翔計算の側は区別しない。

本実装の既定は非粘性の Euler 方程式である。粘性による摩擦抗力、底面抗力、フィン縁の抗力は CFD からは得られず、部品積み上げ法の項で補う（:ref:`sec-cfd-trust`）。
``model = "rans"`` を選ぶと、乱流を Spalart–Allmaras モデルで表す RANS 方程式を、壁面にプリズムの境界層を置いたメッシュで解き、摩擦抗力も CFD から得る（:ref:`sec-cfd-rans-use`）。
RANS は、付着した流れの摩擦と、サンプル機体の :math:`M=0.8`\ 、2、:math:`\alpha=\pm4^\circ` の :math:`C_N`\ 、:math:`C_A`\ 、:math:`x_{cp}`\ （メッシュの収束と Euler との相互比較）まで検証した。
低い亜音速の収束、底面の流れ、CAD のモデルの解は確かめていない。範囲と結果は :ref:`sec-cfd-rans` にあり、係数表に使う前に確かめる。

必要なツールと準備
------------------

CFD モードは次の外部プログラムを呼び出す。それぞれの役割は :ref:`sec-cfd-int-roles` にある。

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - プログラム
     - 用途
   * - SU2_CFD（8.3.0）
     - 流れの解法。Euler 方程式と RANS 方程式を解く :cite:`su2`
   * - gmsh（Python API）
     - 体積メッシュの生成。形状の再構成と、CAD のファイル（STEP、IGES、BREP）の読み込みには OpenCASCADE のカーネルを使う :cite:`gmsh,occt`
   * - Open MPI（``mpirun``）
     - 1 ケースを複数のランクで並列に解くとき（``ranks_per_case`` が 2 以上のとき）に必要 :cite:`openmpi`
   * - manifold3d（Python）
     - STL の閉じた殻の和（ブーリアン演算）。``surface = "stl"`` のときだけ必要 :cite:`manifold`

これらは conda-forge のパッケージとして入手でき、micromamba で一つの環境にまとめて入れるのが簡単である。
例えば次のようにする。

.. code-block:: sh

   micromamba create -n ignisyeet-cfd -c conda-forge python=3.10 su2=8.3.0 gmsh=4.15.2 python-gmsh=4.15.2 openmpi=5

SU2 のバージョンは 8.3.0 に固定している。8.5.0 は AVX-512 命令を持たない CPU で異常終了することが確かめられたためである。

manifold3d は、同じ環境の Python に pip で入れる。版は 3.5.4 に固定している（動作を確かめた版）。

.. code-block:: sh

   <prefix>/bin/python -m pip install manifold3d==3.5.4

インストーラ（``install.sh --cfd``）は、上の版に固定した環境を作ったあとに、これを実行する。インストーラの計画の表示、失敗したときの再実行の案内、入っていないときの案内も、この版を示す。
ツールが見つからないときのエラーにも、``install.sh --cfd`` で入れられることを示す。
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
``surface = "stl"`` と ``"step"`` で ``shape_symmetry`` が ``"auto"`` のときは、鏡映対称が形状を検査するまで分からないので、ケース数は全領域を解く場合の上限として示される。
``--mesh`` を付けると、実際にメッシュを作り、節点数、セル数、壁面の三角形数、セルの種類、品質（SICN の最小値と平均、0.05 未満のセル数）を表示する。
``surface = "stl"`` では形状の忠実度（:ref:`sec-cfd-fidelity`）も、``surface = "step"`` では CAD のモデルの読み込みと位置合わせの結果（:ref:`読み込みと位置合わせの報告 <sec-cfd-step-report>`）も表示する。
メッシュはキャッシュされるので、本番の計算でそのまま再利用される。
必要なツールが足りないとき、``cfd-check`` は不足を報告して終了ステータス 1 で終わる。
CFD モードで ``aero`` を実行したときも、先に同じ検査を行い、ツールが足りなければ計算を始める前にエラーで止まる。
SU2 のバージョンが 8.3.x でないときは、``cfd-check`` も CFD の係数表の作成も、検証した版が 8.3.0 であることを警告として表示する。

.. _sec-cfd-native:

SU2 の高速化ビルド（オプション）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

conda-forge の SU2 8.3.0 は、汎用の x86-64 向けに構築されており、実行する CPU 固有の命令（AVX など）をほとんど使わない。
``install.sh --su2-native`` は、SU2 8.3.0 をソースから、その場の CPU 向けに構築する。
構築は Linux で CFD 環境（``--cfd``）と git、gcc/g++ が必要で、約 10 分かかる。

構築の設定は次のとおりである。

- meson の ``--buildtype=release --optimization=3``。
- C/C++ の引数に ``-march=native -mtune=native``\ （gcc 13 以降の CLI11 のために ``-include cstdint`` も加える）。
- ``-Dwith-mpi=enabled -Dcustom-mpi=true``\ （CFD 環境の Open MPI を使う）、``-Dwith-omp=false``。
- ``-Denable-mixedprec=true``\ （線形代数に単精度を混ぜて、メモリを減らす）。
- ``-Denable-pywrapper=false -Denable-tests=false``。

実行ファイルは ``PREFIX/share/ignisyeet-su2-native/bin/SU2_CFD`` に置かれ、インストーラの最後に ``export IGNISYEET_SU2_CFD=...`` の行としてそのパスが表示される。

**使い方。** 環境変数 ``IGNISYEET_SU2_CFD`` に ``SU2_CFD`` 実行ファイルのパスを入れる。
``[aero.cfd] su2`` が既定の ``"SU2_CFD"`` のままのとき、この環境変数のパスが使われる。
``su2`` に明示的なパスを書いたときは、そちらが優先される。
どちらの場合も、``mpirun``、gmsh、ライブラリは ``[aero.cfd] prefix`` の CFD 環境から取る。
指定しなければ conda-forge の SU2 を使う。

**性能と精度。** 同じ共有ノード（virgo02、EPYC 7742 2 基、16 ランク）で、同じメッシュと設定を冷たい状態から解き、二つのビルドを交互に対にして実行した。
共有ノードなので絶対値は負荷で変わる。比較には対ごとの比（ネイティブ / conda）を使う。
ネイティブ版は、1 反復あたりの時間が Euler で 0.70 〜 0.87 倍、RANS（壁面 5 mm、Mach 0.8、迎角 4°）で 0.75 〜 0.77 倍である。
ピーク RSS（16 ランクの合計）は、Euler で 3.69 GB から約 2.75 GB に、RANS で 7.5 GB から約 5.1 GB に減る。
収束までの反復数は、conda 版と 1 反復ほどしか違わない。

.. list-table:: ネイティブ版 / conda 版の比（対ごと）
   :header-rows: 1
   :widths: 30 14 14 14 14 14

   * - ケース（組）
     - 秒/反復
     - 全体の時間
     - ピーク RSS
     - 反復数（conda、native）
     - 実行順
   * - Euler Mach 0.8, 4°（1）
     - 0.786
     - 0.795
     - 0.747
     - 354、355
     - conda が先
   * - Euler Mach 0.8, 4°（2）
     - 0.866
     - 0.871
     - 0.743
     - 354、356
     - native が先
   * - Euler Mach 0.8, 4°（3）
     - 0.704
     - 0.710
     - 0.742
     - 354、354
     - conda が先
   * - Euler Mach 2, 4°（2）
     - 0.709
     - 0.737
     - 0.742
     - 153、155
     - native が先
   * - Euler Mach 2, 4°（3）
     - 0.722
     - 0.745
     - 0.742
     - 154、155
     - conda が先
   * - RANS Mach 0.8, 4°（1）
     - 0.746
     - 0.735
     - 0.682
     - 404、403
     - native が先
   * - RANS Mach 0.8, 4°（2）
     - 0.766
     - 0.740
     - 0.685
     - 403、404
     - conda が先

Euler の Mach 2 の 1 組目は、conda 版が誤って前のケースから初期化されたため除いた。RANS の 1 組目は、ノード負荷が実行の途中で大きく変わったため、参考値である。
全ての値は ``data/cfd/su2_native_benchmark.csv`` にある。

係数の差は、:math:`C_N` が 0.004 % 以内、:math:`x_{cp}` が 0.011 mm 以内である。
:math:`C_A` は 0 に近い値なので相対差が大きく見え、最大で 0.12 % だが、絶対値では 1 〜 3e-5 である。
残差は、Euler で両者がほぼ一致し、RANS ではネイティブ版の方が 400 反復で約 0.2 〜 0.35 桁高い。ただし、どちらも同じ反復で収束判定を満たす。

-march=native のバイナリは構築した CPU の命令集合に依存し、別の CPU に移すと異常終了することがある。
そのため、既定は conda-forge の SU2 のままで、ネイティブ版は利用者が環境変数で選ぶ。

.. _sec-cfd-surface:

壁面の形状
----------

本節は、CFD が解く機体の壁面を何から作るか、その壁面は入力とどれだけ違うか、そして対称性と計算領域をどう扱うかを述べる。
壁面の形状は ``aero.cfd.surface`` で選ぶ。:numref:`fig-cfd-surface-modes` のように、抽出した寸法から立体を作り直す ``panel_mesh`` と、入力の STL を修復して使う ``stl`` があり、
このほかに、CAD のモデル（STEP、IGES、BREP）の立体をそのまま壁面にする ``step`` がある。
``panel_mesh`` と ``stl`` の壁面は入力の STL と同一ではない。``step`` の壁面は CAD の曲面そのもので、Euler で ``base_treatment = "fairing"`` としたときだけ底面の後ろが変わる。

.. _fig-cfd-surface-modes:

.. figure:: _generated/tikz/cfd_surface_modes.*
   :width: 100%

   壁面の二つのモード。(a) ``panel_mesh`` は抽出した半径分布とフィンの寸法から立体を作り直すので、入力にあるレールボタンなどは含まれない。
   (b) ``stl`` は入力の STL を修復して一つの立体にし、壁面を再メッシュする。Euler では底面にフェアリングを加える。どちらの壁面も入力と同一ではなく、``stl`` ではずれを忠実度の報告で確かめる。

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - 値
     - 内容
   * - ``"panel_mesh"``\ （既定。``"cad"`` も同じ意味）
     - 抽出した半径分布とフィンの寸法から、形状を OpenCASCADE の立体として作り直す（:ref:`sec-cfd-rebuild`）。名前に反して、パネル法の表面メッシュは使わない。形状は回転体と台形フィンに限られ、レールボタンやポッド、非対称な突起は入らない
   * - ``"stl"``
     - 入力の STL を修復し、重なった閉じた殻を一つの立体に統合して壁面にする。レールボタンや非対称な部品も壁面に入る（:ref:`sec-cfd-stl`）。壁面は再メッシュされ、Euler では底面も変わるので、入力と同一ではない。和の表面が鏡映対称と確かめられれば、1/4 領域または半領域だけをメッシュにする
   * - ``"step"``
     - ``step_file`` の CAD のモデル（STEP、IGES、BREP）の立体を、細部（ランチラグ、レールボタン、カメラのポッドなど）を含めてそのまま壁面にする（:ref:`sec-cfd-step`）。
       ``rocket.stl`` と同じ機体であることを位置合わせで確かめる。曲面は CAD の厳密な曲面のままメッシュにする。形状が鏡映対称と確かめられれば、1/4 領域または半領域だけをメッシュにする。RANS でも使える

.. _sec-cfd-stl:

STL を直接解く
~~~~~~~~~~~~~~

``surface = "stl"`` は、STL をそのまま CFD に使うモードである。
STL は、サンプル機体のようにフィンが胴体に埋め込まれた別の閉じた立体として表されていることがあり、そのままでは体積メッシュにできない。
そこで IgnisYeet は、STL の殻を修復して一つの立体にし、その表面を再メッシュして壁面とする。
抽出した寸法には依存しないので、``fin_roll_deg`` は使わず、STL に含まれるフィンの向き（ロール）もそのまま保つ。

処理の流れは次のとおりである（詳細は :numref:`fig-cfd-stl-pipeline`、:doc:`cfd_internals`）。

1. 座標を CFD の座標に変換する。
2. 修復する。点の位置は動かさず、頂点の溶接、縮退した三角形の除去、向きの統一、水密の確認だけを行う。水密でない殻、体積がない殻はエラーにする。
3. Euler のときは底面の処理を行う（:ref:`sec-cfd-stl-base`）。
4. 殻を一つの立体に統合する（和）。和が互いに離れた複数の立体になるとき（フィンが胴体に触れていないなどの入力の誤り）はエラーにする。
5. 和の表面を再メッシュして体積メッシュを作る。``stl_remesh = "classify"``\ （既定）は、gmsh で曲面を作って ``wall_size`` の等方的な三角形に作り直す（フィンの面は ``fin_size``）。``"none"`` は和の三角形をそのまま使うが、サンプルの STL ではフィンの面が少数の大きな三角形でフィンを解像できず、法線力が 3 割ほど低くなる。
6. 形状の忠実度を測る（:ref:`sec-cfd-fidelity`）。

**CFD の壁面は入力と同一ではない。** 殻の和、壁面の再メッシュ、Euler の底面のフェアリングで、壁面は入力の STL から変わる。
ずれの大きさは、``wall_size`` の程度に収まるのが普通だが、保証ではない。結果を使う前に、形状の忠実度の報告（:ref:`sec-cfd-fidelity`）を読んで確かめる。

.. _sec-cfd-stl-base:

底面の処理（Euler）
~~~~~~~~~~~~~~~~~~~

非粘性の流れでは、平らな底面の後ろの流れは定まらない（:ref:`sec-cfd-fairing`）。
STL と CAD のモデル（``surface = "step"``）を Euler で解くときは、``base_treatment`` を **必ず** 指定する（指定がなければエラーになる）。

``"fairing"``
   底面の後ろに、尾部フェアリングの円すいを和に加える。長さは ``tail_fairing`` × 底面半径で、半径は一次関数で小さくなり、底面半径の 10 % で切って平らな面で閉じる（尖った頂点があると gmsh が曲面を作れない）。
   円すいの前面は、胴体の底面の縁の多角形と同じ形で、底面と同一平面に重なるので、和に段差や細長い三角形が残らない。
   **入力の STL を変更する** ので、警告を出す。底面の抗力は経験式（部品積み上げ法の底面抗力）で補う（:ref:`sec-cfd-trust`）。
   ``surface = "step"`` では、OpenCASCADE の厳密な円すい（同じ傾きで、底面半径の 10 % で切る）を、最も大きい平らな底面の後ろに CAD の立体と融合する。**CAD の形状を変更する** ので、同じく警告を出す。
   フィンの後縁が底面と同一平面にあるとき（Basic Finner）は、フィンの後縁も底面の面の一部になるので、円すいは底面の面の広がりではなく、胴体の底面の円の縁の 98 % の半径から始める（縁に重ねると CAD の融合が失敗する）。フィンの後縁の面は底面と同じ面なので、``base`` のマーカーに入り、Euler の軸力には積分しない（その抗力は、部品積み上げ法のフィンの後縁の底面抗力で補う）。

``"flat"``
   STL の形状を変えず、平らな底面を残す。ただし、この Euler の解は収束しないことがある。
   サンプルの STL で試したところ、残差は :math:`10^{-2.5}` 付近で止まり、力の係数 :math:`C_{Fz}` は反復 400 回付近で符号が反転して振動した。
   警告を出し、収束しないケースは失敗として扱う。

どちらの場合も、``base`` マーカは力の積分から外す。RANS は ``surface = "stl"`` では使えない（:ref:`sec-cfd-rans-use`）。
``surface = "step"`` の RANS は ``base_treatment`` を使わず、平らな底面のまま解く（指定しても無視する）。

``panel_mesh`` では、Euler のとき尾部フェアリングを置く。長さは ``tail_fairing``\ （既定 6、単位は底面半径）で指定し、0 にすると平らな底面になる。
RANS は常に平らな底面のままにする。
フェアリングは形状を変えるので、``stl`` の ``"fairing"`` と同じく、底面の抗力は CFD からは得られない。

.. _sec-cfd-fidelity:

形状の忠実度の報告
~~~~~~~~~~~~~~~~~~

STL モードでは、CFD の壁面が入力の形からどれだけずれたかを測って報告する。本節は、その報告の読み方を述べる。測り方は :ref:`sec-cfd-int-fidelity` にある。

報告は、入力、殻の和、CFD の壁面の三つの面について、包含箱、三角形の数、表面積、（定義できるものは）体積を示す。
入力は殻が重なるので体積を定義しない。
面どうしの距離は、サンプリングによる近似で測り、最大値、99 パーセンタイル、二乗平均平方根を示す。比較は次の二つである。

* **CFD の壁面と和**\ ：両方向の距離。メッシュが元の面からずれた量を表す。
* **CFD の壁面と入力**\ ：壁面から入力への片方向の距離。フェアリングは入力にないので、``base`` は含めない。

距離の最大値のうち大きいほうが ``wall_size`` の半分を超えると警告を出し、``max_geometry_error``\ （既定は ``wall_size``）を超えるとエラーにする。
最大値の位置も記録されるので、ずれが意図した変更（フェアリングが底面とつながる位置など）か、再メッシュの失敗かを判断できる。
結果は ``<出力先>/<workdir>/mesh/stl_fidelity.json`` に保存され、``ignisyeet cfd-check 設定ファイル --mesh`` が表示し、``cfd_report.json`` の ``fidelity`` にも入る。
99 パーセンタイルと最大値の差が大きいときは、ずれが局所的である。距離はサンプリングによる近似で、細い特徴の見落としがありうる。

サンプルの STL を ``wall_size`` 2 mm、``base_treatment = "fairing"`` で解いたときは、CFD の壁面は和から ``wall_size`` より小さい距離にあり、
入力からの最大のずれはフェアリングが底面とつながる位置にあって、意図した変更である。この最大値は ``wall_size`` の半分を超えるので警告が出るが、99 パーセンタイルは ``wall_size`` よりはるかに小さい。
三つの面の表と距離の表は :ref:`sec-vaero-cfd-fidelity` に示す。

.. _sec-cfd-step:

CAD のモデル（STEP）を解く
~~~~~~~~~~~~~~~~~~~~~~~~~~

``surface = "step"`` は、``step_file`` に書いた CAD のモデル（STEP、IGES、BREP）の立体を、そのまま CFD の壁面にするモードである。
ランチラグ、レールボタン、カメラのポッド、ノズルのように STL の抽出では扱えない細部も、CAD にあれば壁面に入る。
曲面は三角形に近似せず、OpenCASCADE の厳密な曲面のまま gmsh がメッシュにするので、``stl`` のような再メッシュによるずれはない。manifold3d も使わない。

飛翔計算の形状（基準面積、全長、フィンの寸法、部品積み上げ法の項）は、どの壁面のモードでも ``rocket.stl`` から抽出する。
CFD の壁面と飛翔計算の形状が別の機体にならないように、IgnisYeet は CAD のモデルを ``rocket.stl`` と照合し（位置合わせ）、合わなければメッシュを作る前にエラーにする。
付属の例は ``examples/sample_step.toml`` で、サンプル機体にランチラグ 1 本とレールボタン 2 個を足した ``examples/sample_rocket.step``\ （mm 単位）を解く。
ランチラグは外径 6.35 mm、内径 4.8 mm、長さ 50 mm の管で、全長の 45 % の位置にある。レールボタンは軸の直径 8 mm、つばの直径 11 mm、胴体からの高さ 6 mm で、ランチラグから周方向に 90 度離れている。

**CAD からの書き出し**\ 。次の点を守ってファイルを書き出す。

* **形式**\ ：STEP（``.step``、``.stp``）、IGES（``.iges``、``.igs``）、BREP（``.brep``、``.brp``）。拡張子の大文字と小文字は区別しない。
* **単位**\ ：STEP と IGES は、ファイルが宣言する長さの単位から m に換算して読む。BREP は単位を持たないので m として読む。
  ファイルの宣言が誤っているときや BREP が m でないときは、``step_unit`` で単位を指定する（``"m"``、``"cm"``、``"mm"``、``"in"``、``"ft"``）。
  単位を宣言していない STEP や IGES では ``step_unit`` を使えない。単位を宣言して書き出し直す。
  STL の単位（``rocket.stl_scale``）とは別に扱うので、STL が mm で STEP が inch でもよい。
* **一つの閉じた立体**\ ：機体を一つのソリッド（閉じた立体）として書き出す。複数のソリッドは一つに融合し（内部の面は消える）、離れたままのソリッドが残れば、すべてを壁面にして警告を出す。
  ソリッドのない面の集まりは縫い合わせて立体にするが、閉じた立体にならなければエラーになる。
* **座標系**\ ：``rocket.stl`` と同じ座標系（同じ原点、同じノーズの向き、同じロールの向き）で書き出す。IgnisYeet は、STL を CFD の座標に移す変換（:ref:`sec-cfd-stl`）を、CAD のモデルにもそのまま当てる。
  ``rocket.nose_direction`` は両方に効く。CAD の中でモデルを動かしたり回したりしたあとで STL だけを書き出し直すと、位置合わせで止まる。
* **含めるもの**\ ：流れに触れる外形（ノーズ、胴体、フィン、ランチラグ、レールボタン、ポッド、底面から突き出たノズルなど）。モーターや内部の構造は含めない。
  機体の後端は、STL の機体の後端 :math:`x=L` にある平らな後ろ向きの面（底面）で閉じる。``"fairing"`` は、その平らな面の後ろに円すいを付ける。
  底面より後ろにある面（突き出たノズル）は底面の扱いになり、力を積分しない（:ref:`sec-cfd-domain`）。
* **鋭い縁**\ ：Euler では縁が鋭くてもよい。RANS のプリズム層は、鋭い縁や狭いすき間（ランチラグの穴、レールボタンのつばと胴体の間）に沿えないことがあり、そのときは縁を少し丸めて書き出し直す（下の表）。

**設定**\ 。``[aero.cfd]`` の次の項目を使う（全項目は :ref:`[aero.cfd] の設定項目 <sec-config-aero>`）。

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - 項目
     - 内容
   * - ``surface = "step"``
     - CAD のモデルを壁面にする
   * - ``step_file``
     - CAD のファイル。相対パスは設定ファイルのディレクトリから解釈する。設定を読むときに、ファイルがあることと拡張子を確かめる
   * - ``step_unit``
     - 長さの単位。``"auto"``\ （既定。ファイルが宣言する単位、BREP は m）、``"m"``、``"cm"``、``"mm"``、``"in"``、``"ft"``
   * - ``base_treatment``
     - Euler で **必須**\ 。``"fairing"`` か ``"flat"``\ （:ref:`sec-cfd-stl-base`）。RANS では使わない
   * - ``tail_fairing``
     - ``"fairing"`` の円すいの長さ（底面半径の倍数、既定 6）
   * - ``wall_size`` / ``fin_size``
     - 壁面とフィンの面の要素の大きさ。位置合わせの許容（下記）も ``wall_size`` で決まる
   * - ``shape_symmetry`` / ``symmetry`` / ``z_mirror_mesh``
     - ``stl`` と同じく、形状の検査に通った平面だけを使う（:ref:`sec-cfd-stl-symmetry`）

``step_file`` と ``step_unit`` を ``surface = "step"`` 以外で書くとエラーになる。
CAD のファイルの内容は係数表のハッシュとメッシュのハッシュに入るので、ファイルを書き換えると、係数表もメッシュもケースも作り直す。
メッシュのハッシュはファイルの場所を含まないので、同じ内容のファイルを別の場所に移しても、メッシュとケースは再利用される。
``aero`` などの設定の要約では、``cfd`` の行に ``surface step (sample_rocket.step)`` のようにファイル名が出る。

**処理の流れ**\ （詳細は :ref:`sec-cfd-step-internals`）。

1. ファイルを読み、単位を m にそろえ、STL と同じ変換で CFD の座標に移す。面の種類、面積、重心、法線、最も細い面の幅を調べる。
2. ``rocket.stl`` と照合する（位置合わせ）。
3. 形状が :math:`y=0` と :math:`z=0` について鏡映対称かを、CAD の曲面で検査し、計算領域を決める。
4. Euler の ``"fairing"`` では、底面の後ろに円すいを融合する。
5. 体積メッシュを作る。Euler は四面体、RANS はプリズム層と四面体である。

読み込みと対称性の検査の結果は ``<出力先>/<workdir>/mesh/`` にキャッシュされ、メッシュの設定だけを変えたときは読み込みからやり直さない。

**位置合わせ**\ 。``rocket.stl`` を CFD の座標に移した面と、CAD のモデルの面を比べ、次のすべてを満たすときに同じ機体とみなす。
:math:`L` は STL から抽出した全長、:math:`h_w` は ``wall_size`` である。

* STL の包含箱が、CAD のモデルの包含箱に :math:`0.01L` の余裕で収まる。
* ノーズの先端（:math:`x` の最小値）の差が :math:`0.01L` 以下である。
* CAD のモデルの包含箱が、どの軸でも STL の 1.25 倍（と :math:`0.01L`）を超えない（細部で大きくなるのはこの範囲まで）。
* STL の面の上に選んだ 4 万点が、すべて CAD のモデルの面から :math:`\max(2h_w,\,0.01L)` 以内にある。
  CAD の立体の内側に :math:`\max(h_w,\,0.002L)` より深く埋まった点は除く。胴体に差し込んだ箱で表したフィンのように、STL の中に埋まった面があるためである。
* 除いた点が STL の点の 25 % 以下である（多ければ CAD のモデルが STL より大きい）。

CAD のモデルにあって STL にない部分（STL の面から :math:`\max(h_w,\,0.002L)` より離れた部分）は、CAD の細部として許し、つながった領域の数と面積を報告する。
サンプル機体（:math:`L` = 1.5 m）を ``wall_size`` 2 mm で照合すると、許容は 15 mm、細部とみなす距離は 3 mm である。付属の ``sample_rocket.step`` では、ランチラグと 2 個のレールボタンが 3 つの細部の領域になる。
照合は CFD の座標で行うので、CAD のモデルを書き出し直したときに位置や向きがずれれば、ここで分かる。
許容は形の大きな食い違い（単位、向き、別の版のモデル）を見つけるためのもので、:math:`\max(2h_w,\,0.01L)` より小さい形の違い（フィンの厚さの違いなど）は通る。

.. _sec-cfd-step-report:

**読み込みと位置合わせの報告**\ 。``ignisyeet cfd-check 設定ファイル --mesh`` は、メッシュの統計のあとに次の形の報告を表示する（数値は省略した）。

.. code-block:: text

   CAD surface (surface = "step", base treatment: fairing):
     file        …/sample_rocket.step (step, unit declared 0.001 m, step_unit auto, scale 1)
     shape       1 solid(s) in the file, 1 after fusion; … faces (… base), area … m^2, volume … m^3
     details     3 STEP-only region(s), … m^2; smallest face width … m -> smallest mesh size … m
     domain      full: shape symmetry none (requested auto), full mesh: auto: not symmetric (y = 0: …; z = 0: …)
       y = 0: not symmetric (max mismatch … m, …)
       z = 0: not symmetric (max mismatch … m, …)
     base        fairing {…}
     registration  STL -> STEP surface: max … m at (…), p99 … m, rms … m (… samples, allowed … m; … % of the samples buried inside the STEP solid)
     STEP -> STL: max … m, p99 … m; 3 STEP-only region(s) farther than … m from the STL, … m^2
     bounding box [m]: STL  x xmin..xmax y ymin..ymax z zmin..zmax
                       STEP x xmin..xmax y ymin..ymax z zmin..zmax
     offsets STEP - STL [m]: min (…, …, …), max (…, …, …)

``file`` は単位の宣言と換算の倍率、``shape`` は融合の前後のソリッドの数と面の数、``details`` は CAD だけにある細部と最も細い面の幅、``domain`` は計算領域と対称性の判定、``base`` は底面の処理、``registration`` 以下は位置合わせの距離と包含箱である。
読み込みで縫い合わせや形状の修復をしたときは ``note:`` の行に、離れたソリッドや ``"fairing"`` で形を変えたことは ``warning:`` の行に出る。
同じ内容は ``cfd_report.json`` の ``step`` に入る（:ref:`sec-file-cfd`）。

**対称性と計算領域**\ 。CAD のモデルの鏡映対称は、``stl`` と同じく :math:`y=0` と :math:`z=0` の平面について検査する（方法は :ref:`sec-cfd-step-internals`）。
``shape_symmetry = "auto"`` では検査に通った平面だけを使い、``"axisymmetric"`` や ``"pitch_plane"`` を明示して通らなければエラーにする（:ref:`sec-cfd-stl-symmetry`）。
対称な面からずれたランチラグやレールボタンがあると、形状は鏡映対称でなく、全領域を解く。全領域は半領域の約 2 倍のセルになり、負の迎角も（横滑りを解くときは負の横滑り角も）解く。
付属の ``sample_rocket.step`` は対称でなく、全領域になる。
係数表は常に 6 成分の一般表である（:ref:`sec-cfd-table-kind`）。
両方の平面について対称と確かめられ、1/4 領域を CAD の形状から切り出せないとき（ノーズ先端を通る :math:`y=z=0` の辺のブーリアン演算に OpenCASCADE が失敗したとき）は、半領域をメッシュにして負の迎角も解き、その旨を表示する。

**計算量**\ 。1 ケースのセル数は、``panel_mesh`` と同じく ``wall_size`` と ``fin_size`` でほぼ決まり、全領域ならその約 2 倍である。
例えば ``wall_size`` 6 mm（フィンの面は 1 mm）で、``sample_rocket.step`` の全領域は 162 万セル、同じ大きさの ``panel_mesh`` の半モデルは 81 万セルであった（``doc/data/cfd/fin_resolution.csv``）。
細い面（ランチラグの管の壁など）には、その幅の 1/4 まで（``wall_size`` の 1/8 より小さくはしない）細かい要素を置くので、細部の多いモデルではセルが増える。
このメッシュでの :math:`\alpha=0` の横方向の力の比較は :ref:`フィンの面の大きさの比較 <sec-cfd-val-fin-size>` にある。

**RANS**\ 。``surface = "step"`` は ``model = "rans"`` でも使える。プリズム層を CAD の壁面のメッシュに積み、層の検査（不正なプリズム、体積が 0 以下のセル、マーカのない境界の面がないこと）に通ったときだけメッシュとして使う。
CAD の縁は鈍くしない（``panel_mesh`` の RANS ではフィンの縁を鈍くする）。ランチラグの穴やレールボタンのつばの下のような狭いすき間では、向かい合う壁の層が交わらないように柱を短くする（:ref:`sec-cfd-step-internals`）。
RANS は 1/4 領域を使わず、半領域か全領域でメッシュを作る。
付属の ``sample_rocket.step`` の RANS のメッシュ（``wall_size`` 5 mm、全領域）の試験は、層の検査に通ったメッシュのすべてのセルの体積が正で、層の上面の大きさの比の 99 パーセンタイルが 8 未満であること、通らないときは鋭い縁を示すエラーで止まることを確かめる。STEP の壁面の RANS の解（SU2 の計算）は確かめていない。

**エラーと対処**\ 。``surface = "step"`` に固有のエラーと対処は次のとおりである（``…`` は値やパス）。

.. list-table::
   :header-rows: 1
   :widths: 50 50
   :class: longtable

   * - エラー
     - 原因と対処
   * - ``aero.cfd.surface = "step" needs step_file = "<path>" (the CAD model as STEP, IGES or BREP, relative to the configuration file)``
     - ``step_file`` がない。CAD のファイルを指定する
   * - ``aero.cfd.step_file = "…": not a CAD file OpenCASCADE reads; use a STEP (.step, .stp), IGES (.iges, .igs) or BREP (.brep, .brp) file``
     - 拡張子が読める形式でない。CAD から STEP などで書き出す
   * - ``aero.cfd.step_file = "…": no such file (…; a relative path is relative to the configuration file)``
     - ファイルがない。相対パスは設定ファイルのディレクトリからである
   * - ``aero.cfd.surface = "step" with model = "euler" needs base_treatment: …``
     - Euler で ``base_treatment`` がない。``"fairing"`` か ``"flat"`` を指定する（:ref:`sec-cfd-stl-base`）
   * - ``aero.cfd.step_file and aero.cfd.step_unit are only used with surface = "step"``
     - ほかの ``surface`` で ``step_file`` か ``step_unit`` を書いた。消すか ``surface = "step"`` にする
   * - ``aero.cfd.step_unit = "…": the length unit of … cannot be read from the file, so it cannot be overridden; use step_unit = "auto"``
     - 単位を宣言していない STEP か IGES で ``step_unit`` を指定した。単位を宣言して書き出し直す
   * - ``aero.cfd.surface = "step": … does not describe the same rocket as rocket.stl (compared in the CFD frame, after the transform of the STL):`` に続く理由と ``likely cause: …``
     - 位置合わせに通らない。理由の行（``the nose tips differ``、``the STL extends beyond the STEP along …``、``the STEP is … times as large as the STL along …``、``the STL lies up to … m from the STEP surface …``、``… % of the STL lies inside the STEP solid``）と報告の包含箱を見る。
       ``likely cause: units: …`` は単位（``step_unit`` と ``rocket.stl_scale`` を確かめる）、``a different orientation or position: …`` は向きか原点の違い（STL と同じ座標系で書き出し直す）、``a different revision of the CAD model …`` は別の版のモデルである
   * - ``the import of the CAD file failed (log: …/prepare.log):`` と ``mesh error: … contains no closed solid (… faces could not be sewn into one); export the rocket as a solid body``
     - ファイルに閉じた立体がなく、面を縫い合わせても閉じない。CAD でソリッドとして書き出す
   * - ``mesh error: gmsh cannot mesh the surface of …, even after shape healing (…); repair the CAD model or export it again``
     - 形状の修復をしても表面のメッシュができない。CAD でモデルを直すか、書き出し直す
   * - ``aero.cfd.surface = "step": base_treatment = "fairing" needs a flat aft-facing base face at x = … m (the base of the STL geometry), but the shape has none; use base_treatment = "flat" (the shape is kept as it is)``
     - STL の後端の位置に平らな底面がない。底面を平らにするか、``"flat"`` にする
   * - ``aero.cfd.surface = "step": the shape has no base faces (aft-facing faces at the base x = … m of the STL geometry); the CAD model must end in a flat base``
     - 底面とみなす面がない。機体の後端を平らな面で閉じる
   * - ``shape_symmetry = "axisymmetric" needs a surface that is mirror symmetric about y = 0 and z = 0, but the CAD shape is not: …``\ （``"pitch_plane"`` も同じ形）
     - 明示した対称性が検査に通らない。``"auto"`` か ``"none"`` にする
   * - ``gmsh meshing of the CAD surface failed (log: …/mesh.log):`` と ``mesh error: the prismatic boundary layer is invalid: …; the CAD surface is meshed as it is and its sharp edges (…) are not blunted, which the prism layer cannot always follow: use model = "euler", or round the sharp edges slightly in the CAD model``
     - RANS のプリズム層が検査に通らない。``model = "euler"`` にするか、CAD でフィンの前縁と後縁、ランチラグやレールボタンの縁を少し丸める
   * - ``mesh error: the tetrahedral mesh outside the boundary layer failed (…): the layer top probably intersects itself (narrow gaps such as a lug bore or the space between a button and the body are thinner than the layer); …``
     - RANS の層の上面が狭いすき間で自分自身と交わった。すき間を広げるか埋める、縁を丸める、または Euler にする
   * - ``mesh error: the surface mesh of the CAD faces … has duplicated or degenerate triangles with every surface algorithm``
     - どの表面メッシャでも、その面に重なった三角形や縮退した三角形ができる。CAD でその面を作り直すか、``wall_size`` を変える

**注意**\ 。CFD の壁面は CAD のモデルで、飛翔計算の部品積み上げ法の項（底面抗力、摩擦抗力、Euler で補う項）は ``rocket.stl`` の形状から求める。CAD の細部の抗力のうち CFD が解く圧力の抗力は CFD の軸力に入るので、``rocket.extra_cd`` で二重に数えないようにする（付属の例は 0）。
Euler の摩擦抗力は STL の形状から求めるので、細部の摩擦抗力は入らない。

.. _sec-cfd-stl-symmetry:

対称性と計算領域
~~~~~~~~~~~~~~~~

機体が鏡映対称なら、解く領域を半分、または 1/4 にして、計算量を減らせる。本節は、対称性の指定と、それによって決まる領域の意味を述べる。

計算領域には次の種類がある。

* **全領域**\ ：機体全体を囲む領域。横滑り角の負側やロール減衰のケースは、全領域でなければ解けない。
* **半領域**\ ：機体の片側（:math:`y\ge0`）だけを囲み、:math:`y=0` の平面に対称境界を置く。セル数は全領域の約半分になる。
* **1/4 領域**\ ：:math:`y\ge0` かつ :math:`z\ge0` の領域。メッシュを作る途中の領域で、:math:`z=0`（と必要なら :math:`y=0`）で鏡映して半領域または全領域にしてから解く。

半領域を使うのは、``symmetry = true``\ （既定）で、かつ、形状が :math:`y=0` について対称で、``shape_symmetry`` に :math:`y=0` の鏡映が含まれ、``betas_deg = [0]`` で、ロール減衰のケースがないときだけである。
どれか一つでも崩れると全領域を解く。``symmetry = false`` は全領域を強制する。
``stl`` の対称性は、和の表面を :math:`y=0` と :math:`z=0` の平面について検査して確かめる（検査の方法は :ref:`sec-cfd-stl-symmetry-check`）。
``step`` では、CAD のモデルの曲面を同じ二つの平面について検査する（:ref:`sec-cfd-step-internals`）。

``shape_symmetry`` は、鏡映像として得られるケースを解かずに省く対称性を指定する。

.. list-table::
   :header-rows: 1
   :widths: 26 74

   * - 値
     - 動作
   * - ``"auto"``\ （既定）
     - ``panel_mesh`` では、横滑りを解かないとき ``"axisymmetric"``\ 。横滑りを解くときは、フィンの配置から決める（両面について対称なら ``"axisymmetric"``、:math:`y=0` だけなら ``"pitch_plane"``、それ以外は ``"none"``）。
       ``stl`` と ``step`` では、検査に通った平面だけを使う（両方なら ``"axisymmetric"``\ （既定の ``z_mirror_mesh`` では 1/4 領域、``false`` なら半領域をメッシュにする）、:math:`y=0` だけなら ``"pitch_plane"``\ （半領域）、それ以外は ``"none"``\ （全領域））。:math:`z=0` だけが通ったときは、使える形状の対称性がないので ``"none"`` とする
   * - ``"axisymmetric"``
     - :math:`y=0` と :math:`z=0` の両方について鏡映対称（フィンが 4 枚で ``fin_roll_deg`` が 0 度または 45 度など）。:math:`\alpha\ge0`、:math:`\beta\ge0` だけを解く
   * - ``"pitch_plane"``
     - :math:`y=0` だけについて鏡映対称（フィンが 3 枚など）。:math:`\pm\alpha` と :math:`\beta\ge0` を解く
   * - ``"none"``
     - 対称性を使わない。検査にかかわらず全領域で、:math:`\pm\alpha` と :math:`\pm\beta` を解く

``stl`` と ``step`` で ``"axisymmetric"`` か ``"pitch_plane"`` を明示したとき、形状が検査に通らなければ **エラー** にして、最大の距離と理由を示す。
非対称な形状を黙って鏡映して、鏡映像の係数を誤ることはない。``panel_mesh`` では検査をせず、明示した値をそのまま使うので、フィンの配置と合う値を指定する。
選んだ対称性と理由は、実行のとき標準エラー出力に表示し、``cfd_report.json``\ （``symmetry_check``）に記録する。``stl`` では ``stl_fidelity.json``、``step`` では ``cfd_report.json`` の ``step`` にも入る。
STL の三角形分割が曲面を鏡映の関係にない平面で近似していれば、人の目に対称でも検査に通らず、全領域を解く。許容は変えられない。

フィンは既定では :math:`\pm y` と :math:`\pm z` の 4 方向に付き、:math:`y=0` 面に二枚のフィンが載る。
``fin_roll_deg``\ （既定 0）でフィンの組を機軸のまわりに回転できる（45 度にするとフィンは :math:`y=0` 面に載らない）。この設定は ``panel_mesh`` だけで使う。

サンプルの STL では、検査に通れば 1/4 領域のメッシュが全領域より大幅に速く作れ、セル数も約 4 分の 1 になる。
同じメッシュから作った半領域と鏡映した全領域の係数は近いが、別々に作った二つのメッシュの比較は、メッシュの違いが混ざるので意味を持たない。
数値は :ref:`sec-vaero-cfd-symmetry` に示す。

.. _sec-cfd-table-kind:

係数表の種類
~~~~~~~~~~~~

CFD が作る係数表には、軸対称表と一般表（6 成分、:ref:`sec-aero-coeff6`）の二種類がある。
次のどれかのとき、一般表になる。

* ``surface = "stl"`` か ``"step"`` である。
* ``shape_symmetry`` が ``"pitch_plane"`` か ``"none"`` と明示されている。
* ``betas_deg`` に 0 以外がある。

それ以外は軸対称表で、ロール減衰だけを解いた場合も軸対称表である。
一般表は、:math:`C_X,C_Y,C_Z,C_l,C_m,C_n` を (M, α, β) の関数として持つ 6 成分の準定常空力の表である（:ref:`sec-aero-table`）。
迎角は :math:`-180^\circ` から :math:`180^\circ`、横滑り角は :math:`-90^\circ` から :math:`90^\circ` の格子で、解いた範囲の外はモデルの値で埋める（:ref:`sec-cfd-trust`）。
Mach 数の刻みは ``general_mach_step``\ （既定 0.1、0 なら ``[aero]`` の ``mach_step``）で、粗くするとファイルが小さくなる。
格子と作り方の詳細は :ref:`sec-cfd-general` にある。


.. _sec-cfd-run:

解く流れと条件
--------------

本節は、どの方程式をどの条件の組合せで解くか、そのコストと資源、実行の再開と収束の扱いを述べる。

流れのモデル
~~~~~~~~~~~~

``model`` で流れのモデルを選ぶ。

* **Euler**\ （既定）：非粘性の圧縮性流れ。壁面の力は圧力だけで決まるので、摩擦抗力は含まれない。摩擦、底面、フィン縁の抗力は部品積み上げ法の項で補う。
* **RANS**\ ：Spalart–Allmaras モデルで乱流を含めて解く。壁面の摩擦と底面の圧力も CFD の解に入る。
  ``surface = "panel_mesh"`` と ``"step"`` で使え、壁面にプリズムの境界層を置いたメッシュを作る。使い方と制約は :ref:`sec-cfd-rans-use` にある。

方程式と数値解法は :doc:`cfd_internals` に示す。

(M, α, β) の行列
~~~~~~~~~~~~~~~~

``machs``、``alphas_deg``、``betas_deg`` の組合せがケースの行列である（既定は 10 の Mach 数と 6 の迎角、横滑り角は 0 だけ）。
迎角は 0 を含む必要があり、横滑り角は 0 から始める。
ケースの並び順は、Mach 数が小さい順、各 Mach 数の中で横滑り角（0、正の角度の昇順、（必要なら）負の角度を絶対値の昇順）、その中で迎角を同じ順とする。
``roll_damping`` が真なら、各 Mach 数の最後にロール減衰の 2 ケースが付く（:ref:`sec-cfd-roll-cases`）。
``betas_deg`` が 0 だけでなければ、係数表は一般表になる（:ref:`sec-cfd-table-kind`）。

``alpha_mode`` は、迎角の正負と :math:`\alpha=0` での非対称性の扱いを決める。
式と手順は :ref:`sec-cfd-alpha-mode` にある。

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

一般表では、``"mirror"`` と ``"offset"`` のどちらでも、:math:`\alpha=\beta=0` の横力、法線力、ピッチとヨーのモーメント（:math:`C_Y,C_Z,C_m,C_n`）を全点から引く（:ref:`sec-cfd-general`）。
ロールモーメント :math:`C_l` は、鏡映対称な形状でだけ引く。鏡映対称でない形状では、カントしたフィンのロールの強制力なので残す。
鏡映対称でない形状（ランチラグなど）では、その細部が :math:`\alpha=\beta=0` で生む小さな力も一緒に引かれる。
``"single"`` では、形状の対称性で 0 にならない成分がそのまま残る。

解く符号は ``shape_symmetry``\ （:ref:`sec-cfd-stl-symmetry`）で決まる。

* ``"axisymmetric"``\ ：:math:`\alpha\ge0` と :math:`\beta\ge0` だけ。``alpha_mode = "mirror"`` で :math:`z` 対称メッシュでないときは、各正の迎角を負の迎角でも解く。
* ``"pitch_plane"``\ ：:math:`\pm\alpha` と :math:`\beta\ge0`。
* ``"none"``\ ：:math:`\pm\alpha` と :math:`\pm\beta`。

解かない符号の係数は、後処理で鏡映して補う（:ref:`sec-cfd-general`）。
:math:`n_\alpha` 個の正の迎角と :math:`n_\beta` 個の正の横滑り角を指定したとき、1 つの Mach 数の ``"none"`` のケース数は :math:`(2n_\alpha+1)(2n_\beta+1)` である。
横滑り角のケースは、そのぶん計算が増える。

ロール減衰
~~~~~~~~~~

``roll_damping = true`` にすると、各 Mach 数で :math:`\alpha=\beta=0`、:math:`\hat p=\pm` ``roll_rate_hat``\ （既定 0.01）の 2 ケースを、機体が回る流れ（回転座標系）として全領域で解く。Euler だけで使える。
2 ケースの差からロール減衰微係数 :math:`C_{lp}` を求め、係数表に入れる。差を取るので、フィンのカントや非対称による回転なしのずれは打ち消される。
``roll_damping`` を指定しないときの :math:`C_{lp}` は部品積み上げ法の値である。設定の詳細は :ref:`sec-cfd-roll-cases` と :ref:`sec-cfd-sideslip` にある。

.. _sec-cfd-rans-use:

RANS で解く
~~~~~~~~~~~

``model = "rans"`` は、Spalart–Allmaras モデルの RANS 方程式を、壁面にプリズムの境界層を置いたメッシュで解く。
方程式、メッシュの作り方、数値解法、受け入れの判定は :doc:`cfd_internals` にある。本節は、使うときに知っておくことをまとめる。

**使える条件**\ 。次のどれかに当たると、計算を始める前にエラーになる。

* ``surface = "stl"``\ ：境界層のメッシュは作り直した形状と CAD のモデルにしか作らない（``aero.cfd.surface = "stl" with model = "rans": RANS needs the boundary-layer mesh, which is built only for the CAD surface (surface = "cad") and the CAD B-rep (surface = "step")``）。
* ``surface = "panel_mesh"`` で、フィンの配置が :math:`y=0` について対称でない（``fin_roll_deg`` が 10 度の 4 枚フィンなど）：境界層のメッシュは半領域で作り、全領域はそれを :math:`y=0` で鏡映して作るからである。
  ``surface = "step"`` では、形状が対称でなければ全領域に直接プリズム層を積むので、この制約はない。
  エラーは ``aero.cfd: the RANS boundary-layer mesh needs a fin layout symmetric about the y = 0 plane (fin_roll_deg = 10 with 4 fins is not): use a symmetric fin_roll_deg (for example 0 or 45 with 4 fins) or model = "euler"`` である。
* ``roll_damping = true``\ （``aero.cfd.roll_damping is only implemented for model = "euler"``）。
* ``yplus`` が 5 より大きく 30 より小さい、または 150 より大きい（``aero.cfd.yplus = … lies in the buffer layer, where neither a resolved wall nor a wall function is valid: use yplus <= 5 (resolved wall, recommended 1) or 30..150 (standard wall function)``）。

**既定値**\ 。``model = "rans"`` で次の項目を設定しないときは、Euler と異なる既定値を使う。
``wall_size`` 5 mm、``iterations`` 4000（2 次精度の段の反復数）、``min_iterations`` 400、``stable_window`` 200 である。
``yplus`` の既定は 1 で、壁面まで境界層を解像する。``z_mirror_mesh`` は効かず、``alpha_mode = "mirror"`` では負の迎角も解く（メッシュが :math:`z` について厳密に対称でないため）。
底面は常に平らで、尾部フェアリングは置かない。
``wall_size``\ 、``iterations``\ 、``min_iterations``\ 、``stable_window`` を明示したときは、RANS の既定ではなくその値を使う。
Euler の設定ファイルの ``model`` だけを ``"rans"`` に変えたときは、設定ファイルにある ``wall_size`` と ``iterations`` がそのまま効く。
``examples/sample_rans.toml`` は、これらを設定せず RANS の既定のままにしたサンプル機体の設定である（半モデル、:math:`M=0.3,0.8,1.2,2`\ 、:math:`\alpha=0,\pm4,\pm8^\circ` の 20 ケース、``alpha_mode = "mirror"``\ 、``yplus = 1``）。
ファイルのコメントによると、1 ケースのメモリは ``wall_size`` 5 mm で実測約 7.5 GB（``cfd-check`` の見積もりは約 9 GB）、3 mm で 18 GB であり、20 ケースを 4 ランク、2 ケースずつ同時で解くのに約 4.4 時間（1 ケース 14 から 61 分）かかった。

**警告**\ 。次の場合は、計算を止めずに警告を出す。

* ``wall_size`` が 4 mm より小さい：``aero.cfd: model = "rans" with wall_size = … m, finer than the RANS default 0.005 m: a half-model case of the sample rocket needs about 7.5 GB at 5 mm and 18 GB at 3 mm, and more on a finer mesh; check the estimate with `ignisyeet cfd-check <config> --mesh` (an explicit wall_size overrides the RANS default)``
* ``yplus`` が 30 から 150（壁関数）：``aero.cfd: yplus = … selects the standard wall function (30..150), which is not verified (the RANS verification uses a resolved wall, yplus = 1); use yplus <= 5 unless you check the results yourself``

``surface = "step"`` で使うときの注意（鋭い縁、狭いすき間、確かめた範囲）は :ref:`sec-cfd-step` にある。

**メッシュの大きさ**\ 。境界層の層数と厚さは ``wall_size`` と ``yplus``\ 、解く Mach 数で決まる（:ref:`sec-cfd-prism`）。
サンプル機体の半モデル（``yplus = 1``\ 、Mach 数 0.8 と 2、``fin_roll_deg = 0``）では次のとおりである（``doc/data/cfd/rans_mesh_sweep.csv``）。
どの ``wall_size`` でも層の厚さは上限で決まり、49 層、21.7 mm になる。
メッシュの作成時間は ``cfd-check --mesh`` の全体の時間で、gmsh は 1 スレッドで動かした。
1 ケースのメモリは :eq:`eq-cfd-mem-rans` による 4 ランクの見積もりである。

.. list-table::
   :header-rows: 1
   :widths: 14 14 14 12 16 14 16

   * - ``wall_size``
     - 節点数
     - セル数
     - 層数
     - 境界層の厚さ
     - メッシュの作成時間
     - メモリの見積もり
   * - 8 mm
     - 34.8 万
     - 87 万
     - 49
     - 21.7 mm
     - 13 s
     - 4.3 GB
   * - 6 mm
     - 54.5 万
     - 128 万
     - 49
     - 21.7 mm
     - 18 s
     - 6.7 GB
   * - 5 mm
     - 73.5 万
     - 168 万
     - 49
     - 21.7 mm
     - 22 s
     - 9.0 GB
   * - 4 mm
     - 108.4 万
     - 242 万
     - 49
     - 21.7 mm
     - 30 s
     - 13.2 GB
   * - 3.5 mm
     - 138.5 万
     - 309 万
     - 49
     - 21.7 mm
     - 36 s
     - 16.8 GB
   * - 3 mm
     - 184.5 万
     - 414 万
     - 49
     - 21.7 mm
     - 43 s
     - 22.3 GB
   * - 2.5 mm
     - 257.8 万
     - 582 万
     - 49
     - 21.7 mm
     - 59 s
     - 31.1 GB

``wall_size`` 5 mm のケースを 4 ランクで解いたときの実測のピークは、ランクの合計で約 7.8 GB であった。
全領域は節点数もメモリも約 2 倍になる。メモリの予算（``[resources]`` の ``memory_gb``。既定は使えるメモリ。:ref:`sec-cfd-resources`）に合わせて ``parallel_cases`` が減るので、同時に解けるケースは Euler よりずっと少ない。
``ignisyeet cfd-check 設定ファイル --mesh`` は、境界層の層数、最初の層の高さ、厚さ、不正なプリズムの数（常に 0）も表示する。

**抗力の内訳**\ 。軸力は、壁面（``wall``）の力の積分（圧力と摩擦）に、底面の項、追加抗力、二つの増分を足したものである。
RANS の壁は滑らかなので、設定の粗さの効果を部品積み上げ法の摩擦の差（粗さの増分）として足す。
``panel_mesh`` ではフィンの縁を鈍くした形状を解くので、設定の縁（``fin_le``\ 、``fin_te``）の部品積み上げ法の抗力から、鈍くした縁が持つ分（7.8 %）を引いた値（フィンの縁の増分）を足す（:eq:`eq-cfd-ca-rans`）。
底面の項は ``rans_base`` で選ぶ。``"model"``\ （既定）は部品積み上げ法の底面抗力、``"cfd"`` は CFD が底面（``base``）に積分した力である（:ref:`sec-cfd-post`）。
部品積み上げ法の摩擦抗力そのものは足さない。ケースごとの壁面と底面の軸力は、部品積み上げ法の摩擦と底面の値、二つの増分と並べて ``cfd_report.json`` の ``axial_breakdown`` に記録される。

**結果の状態**\ 。RANS のケースは ``converged``\ （残差が十分に下がり、係数が落ち着いた）か、``unconverged``\ （残差が下がりきらないまま、係数が振動の中で、または残差が止まったまま一定になったので、窓の平均を受け入れた）か、失敗である。
``unconverged`` のケースは ``cfd_report.json`` の ``unconverged`` に載り、警告に係数のばらつきが示される。
受け入れたケースでは、壁面の :math:`y^+` の 95 パーセンタイルを確かめ、3 を超えると警告を出す（判定の詳細は :ref:`sec-cfd-int-rans-accept`）。

計算量と資源
~~~~~~~~~~~~

ケース数は行列の大きさで決まる（数え方は :ref:`sec-cfd-run` の「(M, α, β) の行列」）。
横滑り角や負の迎角のケースは、そのぶん計算が増える。``roll_damping`` は Mach 数ごとに 2 ケースを足す。
1 ケースの重さは、``wall_size`` と ``fin_size`` で決まるセル数による。既定の 2 mm（Euler ではフィンの面は厚さの 1/4、サンプル機体では 1 mm）で、サンプル機体の半モデルは約 142 万セルである。
フィンの面も 2 mm にした約 92 万セルと比べると、計算時間は :math:`M=0.8` と :math:`M=2` の 4 ケースの合計で約 1.9 倍になる（:ref:`フィンの面の大きさの比較 <sec-cfd-val-fin-size>`）。
1 ケースのメモリは :eq:`eq-cfd-mem` の見積もりで 4.8 GB、最も大きいランクのピークの 4 倍で 4.6 GB、4 ランクの合計を 5 s ごとに測った最大で 3.3 GB である。
サンプル機体の 16 ケース（:math:`M=0.3,0.8,1.2,2.0` × :math:`\alpha=0,2,4,8^\circ`）を 4 ランクで 1 ケースずつ解くと約 64 分、``examples/sample_cfd.toml`` の 12 ケースに当たる分は約 49 分であった（別の計算と同時に実行した測定）。
5 mm でもフィンの面は 1 mm のままなので、セル数は約 86 万（2 mm の 6 割）にしか減らず、:math:`C_N` は :math:`M=2` で 1 % ほど低い。全領域は半領域の 2 倍のセルになる。
測定した時間は :ref:`sec-vaero-timing` に示す。

``ranks_per_case``\ （1 ケースの MPI ランク数、既定 4）と ``parallel_cases``\ （同時に動かすケース数、既定 4）の積は、使えるスレッド数以内でなければならない。
どちらも 0 にすると自動になり、ランク数は 4 とスレッド数の小さいほう、同時ケース数は ``スレッド数 / ランク数`` になる。
積が使えるスレッド数を超えるときは、まず同時ケース数を、次にランク数を減らす。

.. _sec-cfd-resources:

``[resources]`` の ``threads`` はスレッド数の上限（MPI ランクの総数と gmsh のスレッド数に効く）で、``memory_gb`` は同時に動かすケースのメモリの予算である。
``memory_gb = 0``\ （既定）のときの予算は、このプロセスが使えるメモリ（Linux の ``MemAvailable``、cgroup の上限、``ulimit -v`` の上限の最小。macOS では物理メモリ）で、正の値を指定するとそれを使う。
予算に収まるまで同時ケース数を減らす（``note: aero.cfd: parallel_cases reduced from … to … (memory budget … GB from …, about … GB per case)``）。
1 ケースの見積もりが予算を超えるときは、メッシュを作ったあと、計算の前にエラーで止まる。
メッセージは ``aero.cfd: a single CFD case needs about … GB (… on … cells / … nodes, … ranks), more than the memory budget of … GB (…); use a coarser mesh (a larger wall_size…) or fewer ranks_per_case (about 40 MB each)…`` で、末尾は ``memory_gb`` を指定していれば ``or raise resources.memory_gb``、既定の予算なら ``or set resources.memory_gb when more memory can be freed for the run`` である。
``ignisyeet cfd-check 設定ファイル --mesh`` は、解決した並列度と見積もりを次の 1 行で表示する（予算が分からない環境では ``memory budget unknown (not checked)``）。

.. code-block:: text

   Resources: 4 ranks x 3 concurrent cases, about 9.0 GB per case (27.0 GB together), memory budget 30.0 GB (available memory, MemAvailable in /proc/meminfo)

1 ケースのメモリは、ランク数 :math:`n_r`、セル数 :math:`N` に対して

.. math::
   :label: eq-cfd-mem

   B \approx 40\ \mathrm{MB}\times n_r+3.3\ \mathrm{kB}\times N

と見積もる（Euler。SU2 8.3.0、四面体メッシュ、4 ランクのうち最も大きいランクのピークの 4 倍から求めた）。ピークの測定値との比較は :ref:`sec-vaero-timing` に示す。
RANS は、SU2 が解と勾配と線形系を節点ごとに持つので、節点数 :math:`N_n` から

.. math::
   :label: eq-cfd-mem-rans

   B \approx 40\ \mathrm{MB}\times n_r+\max\bigl(12\ \mathrm{kB}\times N_n,\ 3.3\ \mathrm{kB}\times N\bigr)

と見積もる。節点あたりの値は、プリズム層と四面体のメッシュで SU2 8.3.0 の RANS（両方の段）のピークを測った 10.3 kB に、約 15 % の余裕を足したものである。
プリズム層のメッシュは節点あたりのセル数が四面体のメッシュ（約 5.5）より少ないので、セルあたりの値では見積もりが大きくなりすぎる。
同じメッシュの Euler の見積もりを下回ることはない。``nice`` は子プロセスにも引き継がれる。

実行、再開、収束の扱い
~~~~~~~~~~~~~~~~~~~~~~

各ケースは ``<出力先>/<workdir>/m0.800_a04.00/`` のようなディレクトリに入り、終わると結果と入力のハッシュを書いた ``done.json`` が残る。
同じハッシュの ``done.json`` があるケースは次の実行で飛ばすので、途中で止めても、設定を一部変えても、影響を受けないケースは再利用される。
最初からやり直すには、このディレクトリを削除する。Ctrl-C や SIGTERM を受けると、子プロセスをまとめて終了し、終わったケースの結果は残す。

Euler のケースの結果は、次のいずれかで扱う。

* **収束**\ ：残差が目標に達し、係数も落ち着いた。
* **早期終了**\ ：反復の途中で残差が十分に下がり、力とモーメントが落ち着いたので、止めて受け入れた（``status`` は ``accepted``）。
  落ち着きの判定は、軸力・法線力・ピッチングモーメントに加えて、ロール減衰のケースでは転がりモーメントの係数 CMx を、横滑りのケースでは横力 CFy とヨーイングモーメント CMz も見る（:ref:`sec-cfd-int-convergence`）。
* **受け入れ**\ ：反復の上限か時間の上限に達したが、残差がある程度下がり、最後の窓で力とモーメントがほぼ一定のとき、窓の平均を係数として受け入れる。残差が目標に達していても、係数がまだ傾いているか振動している状態で上限に達したときも、窓の平均を使う（警告は ``… coefficients averaged over the last N iterations`` で終わる）。「収束」には数えず、``cfd_report.json`` の ``unconverged`` に載る。
  早期終了で止めたケースは、止めると決めた時点の履歴と判定で扱う。
* **失敗**\ ：SU2 のエラー、発散、収束も安定もしていない場合。表の作成では、失敗したケースを飛ばして補間する。

RANS のケースは、``converged``、``unconverged``\ （振動や残差の停滞のまま係数が一定で、窓の平均を受け入れた）、失敗のいずれかになる（:ref:`sec-cfd-rans-use`）。
停滞のケースは ``cfd_report.json`` の ``stalled_cases`` に残差の下がりとともに載り、実行の最後に警告を出す。``cfd_cases.csv`` の ``rans_verdict`` と ``residual_drop`` で確かめる。
発散したケースは、反復の上限を待たずに止め、一度だけ控えめな設定で解き直す。

判定の条件は :ref:`sec-cfd-int-convergence` と :ref:`sec-cfd-int-rans-accept` にある。平らな底面の Euler（STL と CAD のモデルの ``base_treatment = "flat"``）は収束しないことがあり、そのときは失敗として扱う。
失敗したケースは、実行の最後に ``error: aero.cfd: case m0.800_a04.00 failed: <理由>`` の形で 1 行ずつまとめて表示する。
SU2 がシグナル 9 で止められたときは、理由に ``SU2 was killed by signal 9 (SIGKILL; most likely out of memory: about … GB per case with … ranks and … concurrent cases; lower parallel_cases or ranks_per_case)`` と、メモリ不足の可能性と 1 ケースのメモリの見積もりを示す。

出力の見方
----------

本節は、CFD の実行が残すファイルを、どれを見て何を判断するかの順に述べる。全項目は :doc:`file_formats` にある。

``cfd_cases.csv`` には全ケースの状態、反復数、残差、係数（6 成分を含む）、計算時間が入る。
まず ``status`` を見て、``accepted``\ （Euler）や ``unconverged``\ （RANS）と失敗のケースがないか確かめる。
RANS では、各ケースのディレクトリの ``done.json`` に、壁面の :math:`y^+` の 95 パーセンタイル（``yplus_p95``）、受け入れの基準にした残差の最大値（``reference_peak``）、壁面と底面の軸力（``axial``）、警告（``warnings``）も入る（:ref:`sec-file-cfd`）。
``cfd_report.json`` には、次の項目が入る。

* 失敗と未収束の一覧（``failed``、``failed_cases``、``unconverged``）、補間で埋めたケース（``filled``）、警告（``warnings``）、メッシュの大きさと総計算時間。
* 係数表の種類（``table_kind``）、形状の対称性と領域（``shape_symmetry``、``domain``）、横滑り角、一般表で CFD の値を使う範囲（``cfd_region``）。
* 軸対称表では、:math:`\alpha=0` の測定された非対称性（``asymmetry``）。
* :math:`\alpha=\beta=0` の横方向の力（``zero_angle``）。Mach 数ごとに :math:`C_Y`\ （一般表）、:math:`C_Z`、:math:`\alpha=4^\circ` の法線力 :math:`C_{N\alpha}\cdot4^\circ`\ （``cn_ref``）、その比（``ratio``）と、係数表に残る分の比（``ratio_kept``）が入る。
  対称な機体では 0 になるはずの値で、比が 5 % を超えると警告を出す（原因は :ref:`sec-cfd-val-asym-cause`）。
* ロール減衰の CFD とモデルの値と採用した側（``clp``、``clp_source``）、基準のカント（``fin_cant_ref_deg``）。
* 底面の処理（``base_treatment``）、STL の形状の忠実度（``fidelity``）、鏡映対称の検査と選んだ領域（``symmetry_check``）、CAD のモデルの読み込みと位置合わせ（``step``）。
* RANS では、最初の層の :math:`y^+` の見積もり（``est_wall_yplus``）と、ケースごとの軸力の内訳（``axial_breakdown``）。

:numref:`fig-cfd-convergence` に収束の履歴の例を示す。
SU2 の密度の残差（RMS の常用対数）が :math:`10^{-6}` に達するまで、:math:`M=0.8` では 100 反復ほど、:math:`M=2` では 81 反復と 101 反復かかった。
:math:`M=2` では残差が 30 反復ほどで :math:`10^{-5}` に達したあと、:math:`10^{-6}` までに 50〜70 反復かかるが、揚力係数は 20 反復ほど（:math:`M=0.8` では 35 反復ほど）で最終値の 1 % 以内に落ち着いている。
残差が下がりきらなくても、係数が一定になっていれば、早期終了や受け入れで使える。

.. _fig-cfd-convergence:

.. figure:: _generated/plots/cfd_convergence.*
   :width: 100%

   サンプル機体の収束の履歴（Euler、``wall_size`` 2 mm、フィンの面 1 mm、142 万セル、一様流からの開始）。左は密度の残差、右は :math:`\alpha=4^\circ` の揚力係数。


.. _sec-cfd-flow:

サンプル機体の流れ場
--------------------

係数表の値の裏にある流れ場を、サンプル機体の :math:`\alpha=4^\circ` で示す。
条件は :math:`M_\infty=0.8` と :math:`2.0`、メッシュは既定の 2 mm（フィンの面は ``fin_size = "auto"`` で 1 mm。142 万セル）、モデルは Euler で、一様流から解いた。
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
  先端から 0.02〜0.25 m の範囲で、風上側（:math:`z<0`）と風下側（:math:`z>0`）の半分ごとに面積で重み付けした平均は、
  :math:`M_\infty=0.8` で風上側 :math:`-0.015`、風下側 :math:`-0.069`、:math:`M_\infty=2.0` で風上側 0.104、風下側 0.045 である。
* :math:`M_\infty=0.8` のフィンでは、前縁の線上と後縁付近が正圧（橙の線は主に後縁）で、前縁のすぐ後ろは負圧になる。
  :math:`M_\infty=2.0` の鋭い縁に見える値は、縁の 1 つの節点の値で、面の分布を代表しない。
* :math:`M_\infty=0.8` では、尾部フェアリングの付け根（底面の縁）で流れが曲がって加速し、環状の負圧ができる（最小で :math:`C_p\approx-0.9`）。

.. _fig-cfd-cp-surface:

.. figure:: _generated/plots/cfd_cp_surface.*
   :width: 100%

   壁面の圧力係数（:math:`\alpha=4^\circ`、上は :math:`M_\infty=0.8`、下は :math:`M_\infty=2.0`）。左はノーズ、右はフィンと尾部フェアリング。
   色の範囲は図ごとに異なる。


.. _sec-cfd-trust:

どこまで信頼してよいか
----------------------

本節は、CFD が出す係数表のどの値が CFD の結果で、どの値がモデルで補った値かを示し、結果をどこまで信頼してよいかをまとめる。

CFD が与える値
~~~~~~~~~~~~~~

解いたケースの壁面の圧力の積分から、次の値が得られる。

* 法線力 :math:`C_N`（機体軸の :math:`C_Z`）と、ノーズまわりのモーメントから求める圧力中心 :math:`x_{cp}`。
* 軸力のうち圧力による抗力（造波抗力）。RANS では、壁面の摩擦抗力と、``rans_base = "cfd"`` のときは底面の抗力も。
* 横滑りを解けば、横力 :math:`C_Y` とヨーモーメント :math:`C_n`。
* ``roll_damping`` を指定すれば、ロール減衰微係数 :math:`C_{lp}`。回転座標系の Euler の定常解の差から求める。

モデルで補う値
~~~~~~~~~~~~~~

次の値は CFD が直接与えるものではなく、部品積み上げ法（またはパネル法）のモデルで補う。

* Euler では、摩擦抗力、底面抗力、フィン縁とボートテールの抗力（Euler は粘性を含まないため）。ただし ``surface = "stl"`` と ``"step"`` では、フィンの前縁と後縁の面が CFD の壁面にあり、その圧力抗力が CFD の軸力にすでに入っているので、フィン縁の項は加えない。``panel_mesh`` では加える（:ref:`sec-cfd-post`）。
  RANS では、追加抗力、``rans_base = "model"``\ （既定）のときの底面抗力、設定の粗さによる摩擦の増分、``panel_mesh`` のときの設定のフィンの縁による増分を足す（:eq:`eq-cfd-ca-rans`）。
* ピッチとヨーの減衰 :math:`C_{mq}`、:math:`C_{nr}`。定常な計算は与えないので、モデルの和のばらつきを残し、CFD の傾きと重心に合わせて取り替える（:ref:`sec-aero-damping`）。
* ロールのカント強制 :math:`C_{l\delta}`。
* ロールとピッチ・ヨーの交差する微係数は、どの手法でもモデルにない。

``roll_damping`` を指定しても、ロールの 2 ケースの片方が失敗した Mach 数、または結果が有限でない、フィンがあるのに正である、ほぼ 0 である Mach 数では、CFD の値を採用せず、部品積み上げ法の :math:`C_{lp}` を使う。
表の :math:`C_{lp}(M)` は、部品積み上げ法の :math:`C_{lp}(M)` に、採用した Mach 数での CFD と部品積み上げ法の比を Mach 数について補間してかけた値で、採用した Mach 数の範囲の外では端の比で一定とする。採用した側は ``clp_source`` に、各 Mach 数の理由は ``clp`` に記録される（規則は :ref:`sec-cfd-roll-post`）。

CFD の領域の外
~~~~~~~~~~~~~~

一般表で、解いた迎角と横滑り角の範囲の外側の値は、CFD の結果ではなくモデルの値である。
モデルの値は、部品積み上げ法の軸対称表を展開したものを、解いた範囲の境界の CFD の値に合わせて延ばしたもので、非対称な形状の外側では当てにならない。
どこまでが CFD の値かは、``cfd_report.json`` の ``cfd_region`` と ``model_outside_cfd_region`` で確かめる（:ref:`sec-cfd-region-outside`）。
Mach 数の方向には、``extrapolation`` にかかわらず外挿しない。解いた Mach 数の範囲の外では、表は最小または最大の Mach 数の値で一定になる。
表の Mach 数の格子が解いた範囲より広いときは、``cfd_report.json`` の ``mach_clamped_above`` と ``mach_clamped_below``\ （一定になる境界の Mach 数。範囲の内側に収まっていれば項目はない）に記録し、同じ内容の警告を出す。
軸対称表の迎角の方向は、解いた最大の迎角 :math:`\alpha_e` より大きい行を、部品積み上げ法の値へつなぐ（:math:`\alpha_e` で CFD の値、:math:`\min(2\alpha_e,\,90^\circ)` から先は部品積み上げ法の法線力と圧力中心。:ref:`sec-cfd-table-build`）。端の傾きを延長しないので、大きな迎角で圧力中心が機体の前方へ動き続けることはない。

表の意味と限界
~~~~~~~~~~~~~~

CFD の一般表は、(M, α, β) の点ごとに解いた定常解を並べた、6 成分の準定常空力の表である。角速度に依存する非定常な空力を、すべて表現するものではない。
既定の CFD は非粘性の Euler 方程式の解であり、境界層の摩擦やはく離は含まない。
RANS は摩擦と粘性の圧力を含む。検証したのは付着した流れの摩擦と、サンプル機体の :math:`M=0.8`\ 、2、:math:`\alpha=\pm4^\circ` の係数で、底面の流れと低い亜音速の収束は確かめていない（:ref:`sec-cfd-rans`）。
RANS の壁は滑らかで、設定の粗さは部品積み上げ法の摩擦の増分として、``panel_mesh`` の設定のフィンの縁は部品積み上げ法のフィンの縁の項の増分として足す。
メッシュ収束は外挿では確かめていない（:ref:`sec-vaero-cfd-mesh`）。既定の 2 mm と最も細かい 1.8 mm の :math:`C_N` の差は、:math:`M=0.8` で 0.01 %、:math:`M=2` で 0.08 % である（フィンの面はどちらも 1 mm）。
:math:`M=2` では最も細かいメッシュでも :math:`C_N` がわずかに増え、:math:`M=0.8` は単調に収束しない。既定の残差の目標 :math:`10^{-6}` で止めた亜音速の :math:`C_N` は、残差 :math:`10^{-9}` まで解いた値より最大 1.4 % 低く、初期値（暖機起動の経路）で 0.9 % ほど変わる。
STL のメッシュは作成のたびに少し変わるので、:math:`C_N` には、この差を超える不確かさがありうる。
実測との比較（妥当性確認）は Basic Finner の超音速の射場試験との比較だけで（:doc:`validation`）、Euler の :math:`C_{N\alpha}` は実測より 2–10 % 小さく、:math:`x_{cp}` は 0.2 口径以内で合う。部品積み上げ法やパネル法との相互比較は :ref:`sec-vaero-compare` に示す。

確認済み
--------

CFD モードについて、次の点を確かめた。数値と手順は :ref:`sec-vaero-cfd` に示す。

* 円すいの表面圧力は、メッシュを細かくすると理論解に近づく。
* :math:`z` 対称メッシュと ``alpha_mode = "mirror"`` で、:math:`\alpha=0` の見かけの非対称性が消える。
* ロール減衰と横滑りの符号は、軸対称表を展開した値と一致する。
* 亜音速の係数は、CFD とパネル法がよく一致する（手法どうしの相互比較）。
* STL をそのまま解く計算は、作り直した形状による計算と相互比較した。
* CAD のモデル（``sample_rocket.step``）の全領域のメッシュ、単位の誤りの検出、対称な CAD のモデルの 1/4 領域と半領域、Basic Finner の STEP の対称性の判定を、gmsh で試験した（メッシュは SU2 で 1 反復だけ解いて読めることを確かめた）。
* RANS の平板の摩擦は、NASA TMR の SA の解と 0.8 % 以内、:math:`M=2` の van Driest II と 1.2 % 以内で一致し、円すいとフィンなし胴体の摩擦の軸力も平板の見積もりと 2.5〜7.4 % で一致する。
* サンプル機体の RANS の :math:`C_N`\ 、:math:`C_A`\ 、:math:`x_{cp}`\ （:math:`M=0.8,2`\ 、:math:`\alpha=4^\circ`）は、メッシュに対して単調に収束し（GCI 2 % 以内）、Euler の CFD と :math:`C_N` で 3.6〜5.1 %、:math:`x_{cp}` で 4 mm 以内で一致する（相互比較）。

注意
----

実装上の制約は :ref:`sec-lim-cfd` にまとめた。使うときに特に注意する点は次のとおりである。

* CFD の壁面は、``stl`` でも入力と同一ではない。形状の忠実度の報告を読む。``step`` の壁面は CAD のモデルで、``rocket.stl`` との位置合わせの報告を読む。
* RANS は ``surface = "panel_mesh"``\ （フィンの配置が :math:`y=0` について対称なときだけ）と ``"step"`` で使える。1 ケースのメモリが大きい（:ref:`sec-cfd-rans-use`）。
  ``"step"`` の RANS は、メッシュの検査までを確かめ、SU2 の解は確かめていない。
  RANS の亜音速の :math:`\alpha=0` には、底面の後流の非対称による :math:`C_N`\ （:math:`M=0.8` で 0.01〜0.03）が残るので、``alpha_mode = "mirror"``\ （既定）で解く。
  :math:`M=0.3` では :math:`\alpha=0` と負の迎角のケースが収束せず、係数の平均を受け入れる（``unconverged``）。``rans_base = "cfd"`` の底面の力は確かめていない。
  検証の項目と結果（:ref:`sec-cfd-rans`）を確かめてから係数表に使う。
* Euler の底面は尾部フェアリング（``"fairing"``）で、形状が変わる。``"flat"`` は収束しないことがあり、サンプルの STL では収束しなかった。
* 一般表の CFD の領域の外はモデルの値である。
* ピッチ・ヨー減衰は CFD が直接与えるものではなく、交差する微係数は扱わない。
* 計算コストは大きい。ケースの結果（``done.json``）は SU2 の設定とメッシュのハッシュで判定するので、後処理（表の作成）だけが変わっても、ケースは計算し直さない。
  SU2 の設定やメッシュの作り方が変わるコードの更新のあとは、``workdir`` を削除して計算し直す。
