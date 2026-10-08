.. _page-reference-config-cfd:

CFD の設定
================

``method = "cfd"`` のときだけ使う。SU2、gmsh、Open MPI が必要で、``surface = "stl"`` では manifold3d も要る。キーは ``examples/sample_cfd.toml`` の並び（流れと行列、壁面と形状、メッシュ、収束と解法、資源）で載せる。RANS の既定値は ``model = "rans"`` を選んだときの値である。

.. _cfg-aero-cfd:

``[aero.cfd]``
--------------

.. _cfg-aero-cfd-model:

``aero.cfd.model``
^^^^^^^^^^^^^^^^^^

:型: 文字列
:既定値: ``"euler"``
:値/範囲: ``"euler"`` ``"rans"``
:説明: 支配方程式。``"euler"`` は非粘性の Euler 方程式（粘性の抗力項は部品積み上げ法で補う）、``"rans"`` は Spalart–Allmaras モデルの RANS で、壁面にプリズムの境界層を置いたメッシュで解く。RANS は ``surface = "panel_mesh"``\ （フィンの配置が :math:`y=0` について対称なときだけ）と ``"step"`` で使え、``roll_damping`` とは組み合わせられない。``wall_size`` ``iterations`` ``min_iterations`` ``stable_window`` の既定値が変わる。
:関連: :ref:`page-theory-cfd-choose`、:ref:`sec-cfd-rans-use`、:ref:`IY-C122`

.. _cfg-aero-cfd-machs:

``aero.cfd.machs``
^^^^^^^^^^^^^^^^^^

:型: 数値の配列
:既定値: ``[0.3, 0.6, 0.8, 0.95, 1.1, 1.3, 1.6, 2.0, 2.5, 3.0]``
:値/範囲: 空でない、各要素は正、昇順、重複なし
:説明: 解く Mach 数。
:関連: :ref:`IY-C100`、:ref:`IY-C101`

.. _cfg-aero-cfd-alphas_deg:

``aero.cfd.alphas_deg``
^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値の配列
:既定値: ``[0, 2, 4, 8, 12, 16]``
:値/範囲: 各要素は 0 以上、昇順、重複なし、0 を含み、0 より大きい値が 1 つ以上
:説明: 解く迎角 [deg]。
:関連: :ref:`IY-C102`、:ref:`IY-C104`

.. _cfg-aero-cfd-alpha_mode:

``aero.cfd.alpha_mode``
^^^^^^^^^^^^^^^^^^^^^^^

:型: 文字列
:既定値: ``"mirror"``
:値/範囲: ``"mirror"`` ``"offset"`` ``"single"``
:説明: 迎角の正負と :math:`\alpha=0` のずれの扱い。``"mirror"`` は正負の解の奇関数・偶関数部分を使う（:math:`z` 対称メッシュでは負の迎角は解かない）。``"offset"`` は :math:`\alpha=0` の値を引く。``"single"`` は結果をそのまま使う。
:関連: :ref:`sec-cfd-alpha-mode`

.. _cfg-aero-cfd-betas_deg:

``aero.cfd.betas_deg``
^^^^^^^^^^^^^^^^^^^^^^

:型: 数値の配列
:既定値: ``[0]``
:値/範囲: 空でない、[0, 90] の範囲、昇順、重複なし、0 で始まる
:説明: 解く横滑り角 [deg]。``[0]`` 以外にすると係数表は 6 成分の一般表になる。
:関連: :ref:`sec-cfd-table-kind`、:ref:`IY-C110`

.. _cfg-aero-cfd-roll_damping:

``aero.cfd.roll_damping``
^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 真偽値
:既定値: ``false``
:値/範囲: ``true`` ``false``\ （RANS では ``false``）
:説明: 各 Mach 数でロール減衰の 2 ケースを回転座標系で解き、:math:`C_{lp}` を CFD の値にする。Euler だけで使える。
:関連: :ref:`sec-cfd-run`、:ref:`sec-cfd-roll-cases`、:ref:`IY-C125`

.. _cfg-aero-cfd-roll_rate_hat:

``aero.cfd.roll_rate_hat``
^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0.01``
:値/範囲: 0 より大きく 0.05 以下
:説明: ロール減衰のケースの無次元ロール角速度 :math:`\hat p=pd/(2V)`。
:関連: :ref:`IY-C113`

.. _cfg-aero-cfd-surface:

``aero.cfd.surface``
^^^^^^^^^^^^^^^^^^^^

:型: 文字列
:既定値: ``"panel_mesh"``
:値/範囲: ``"panel_mesh"``\ （別名 ``"cad"``） ``"stl"`` ``"step"``
:説明: 壁面の形状。``"panel_mesh"`` は抽出した胴体とフィンを再構成する。``"stl"`` は入力の STL から壁面を作る（入力の形状と同一ではない）。``"step"`` は ``step_file`` の CAD のモデルをそのまま壁面にする（``rocket.stl`` と照合する）。``"stl"`` と ``model = "rans"`` の組合せはエラーである。
:関連: :ref:`sec-cfd-surface`、:ref:`sec-cfd-stl`、:ref:`sec-cfd-fidelity`、:ref:`sec-cfd-step`、:ref:`IY-C122`

.. _cfg-aero-cfd-step_file:

``aero.cfd.step_file``
^^^^^^^^^^^^^^^^^^^^^^

:型: パス
:既定値: なし
:値/範囲: STEP（``.step`` ``.stp``）、IGES（``.iges`` ``.igs``）、BREP（``.brep`` ``.brp``）
:説明: ``surface = "step"`` で必須。CAD のファイル。相対パスは設定ファイルのディレクトリを基準にする。``rocket.stl`` と同じ座標系の、一つの閉じた立体として書き出す。``surface = "step"`` 以外ではエラーである。
:関連: :ref:`sec-cfd-step`、:ref:`page-howto-cad-step`、:ref:`IY-C117`、:ref:`IY-C118`、:ref:`IY-C120`

.. _cfg-aero-cfd-step_unit:

``aero.cfd.step_unit``
^^^^^^^^^^^^^^^^^^^^^^

:型: 文字列
:既定値: ``"auto"``
:値/範囲: ``"auto"`` ``"m"`` ``"cm"`` ``"mm"`` ``"in"`` ``"ft"``
:説明: ``step_file`` の長さの単位。``"auto"`` はファイルが宣言する単位（BREP は m）で読む。単位を宣言していない STEP と IGES では ``"auto"`` を指定できない。``surface = "step"`` 以外ではエラーである。
:関連: :ref:`sec-cfd-step`、:ref:`IY-C120`

.. _cfg-aero-cfd-base_treatment:

``aero.cfd.base_treatment``
^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 文字列
:既定値: なし
:値/範囲: ``"fairing"`` ``"flat"``
:説明: ``surface = "stl"`` または ``"step"`` で Euler のとき必須。``"fairing"`` は底面の後ろに円すいを足す（入力の STL や CAD のモデルを変更する）。``"flat"`` は平らな底面のままにする（Euler の解は収束しないことがあり、収束しないケースは失敗として扱う）。``"fairing"`` は ``tail_fairing`` が正のときだけ指定できる。
:関連: :ref:`sec-cfd-stl-base`、:ref:`IY-C116`、:ref:`IY-C119`、:ref:`IY-C121`

.. _cfg-aero-cfd-stl_remesh:

``aero.cfd.stl_remesh``
^^^^^^^^^^^^^^^^^^^^^^^

:型: 文字列
:既定値: ``"classify"``
:値/範囲: ``"classify"`` ``"none"``
:説明: ``surface = "stl"`` の壁面の処理。``"classify"`` は gmsh で曲面を作り ``wall_size`` で作り直す。``"none"`` は STL の三角形をそのまま使う。
:関連: :ref:`sec-cfd-stl`

.. _cfg-aero-cfd-max_geometry_error:

``aero.cfd.max_geometry_error``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``wall_size``
:値/範囲: 正
:説明: ``surface = "stl"`` で、CFD の壁面が入力の STL から離れてよい最大の距離 [m]。超えるとエラー、``wall_size`` の半分を超えると警告する。
:関連: :ref:`sec-cfd-fidelity`、:ref:`IY-C115`

.. _cfg-aero-cfd-symmetry:

``aero.cfd.symmetry``
^^^^^^^^^^^^^^^^^^^^^

:型: 真偽値
:既定値: ``true``
:値/範囲: ``true`` ``false``
:説明: ピッチ面（:math:`y=0`）で切った半モデルを使ってよいか。条件を満たさないときと ``false`` は全領域を解く。
:関連: :ref:`sec-cfd-stl-symmetry`

.. _cfg-aero-cfd-shape_symmetry:

``aero.cfd.shape_symmetry``
^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 文字列
:既定値: ``"auto"``
:値/範囲: ``"auto"`` ``"axisymmetric"`` ``"pitch_plane"`` ``"none"``
:説明: 鏡映像として得て解かない対称性。``"axisymmetric"`` は :math:`y=0` と :math:`z=0`、``"pitch_plane"`` は :math:`y=0`、``"none"`` は使わない、``"auto"`` は形状から判定する。
:関連: :ref:`sec-cfd-stl-symmetry`、:ref:`sec-cfd-table-kind`

.. _cfg-aero-cfd-z_mirror_mesh:

``aero.cfd.z_mirror_mesh``
^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 真偽値
:既定値: ``true``
:値/範囲: ``true`` ``false``
:説明: 1/4 領域をメッシュにして :math:`z=0` で鏡映し、:math:`z` について厳密に対称なメッシュにするか。形状が :math:`z` について対称でないときと RANS では無視する。
:関連: :ref:`sec-cfd-stl-symmetry`

.. _cfg-aero-cfd-fin_roll_deg:

``aero.cfd.fin_roll_deg``
^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0``
:値/範囲: 有限
:説明: フィンの組を機軸のまわりに回す角度 [deg]。0 ではフィン 2 枚が :math:`y=0` 面に載り、45 では載らない。
:関連: :ref:`IY-C126`

.. _cfg-aero-cfd-general_mach_step:

``aero.cfd.general_mach_step``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0.1``
:値/範囲: 0 以上
:説明: 一般表の Mach 数の刻み。0 は ``aero.mach_step`` を使う。
:関連: :ref:`sec-aero-table-format`、:ref:`IY-C114`

.. _cfg-aero-cfd-tail_fairing:

``aero.cfd.tail_fairing``
^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``6.0``
:値/範囲: 0 以上
:説明: Euler のみ。平らな底面の後ろに置く円すい形の尾部フェアリングの長さ（底面半径の倍数）。0 は平らな底面。RANS は常に平らな底面である。
:関連: :ref:`sec-cfd-fairing`、:ref:`IY-C127`

.. _cfg-aero-cfd-farfield:

``aero.cfd.farfield``
^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``20.0``
:値/範囲: 5 以上
:説明: 遠方境界の半径（全長の倍数）。半球と球の半径、または箱の半辺。
:関連: :ref:`sec-cfd-domain`、:ref:`IY-C128`

.. _cfg-aero-cfd-wall_size:

``aero.cfd.wall_size``
^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0.002``\ （RANS は ``0.005``）
:値/範囲: 正
:説明: 壁面の要素の大きさ [m]。RANS では境界層の上面の四面体の大きさでもあり、層の数と厚さが決まる。
:関連: :ref:`sec-cfd-prism`、:ref:`IY-C106`

.. _cfg-aero-cfd-fin_size:

``aero.cfd.fin_size``
^^^^^^^^^^^^^^^^^^^^^

:型: 数値または文字列
:既定値: ``"auto"``
:値/範囲: ``"auto"``、または ``wall_size``/50 以上 ``wall_size`` 以下の長さ [m]
:説明: フィンの面の要素の大きさ [m]。``"auto"`` は Euler で ``wall_size`` とフィンの厚さの 1/4 の小さいほう、RANS で ``wall_size``。数値を書くと両方のモデルでその大きさにする。
:関連: :ref:`sec-cfd-val-asym-cause`、:ref:`IY-C107`、:ref:`IY-C108`、:ref:`IY-C109`

.. _cfg-aero-cfd-yplus:

``aero.cfd.yplus``
^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``1.0``
:値/範囲: 5 以下（壁面まで解像）、または 30 以上 150 以下（標準の壁関数）
:説明: RANS の最初の層の目標 :math:`y^+`\ （全長の 5 % の位置、最大の Mach 数で決める）。その間と 150 を超える値はエラーである。
:関連: :ref:`sec-cfd-prism`、:ref:`IY-C123`

.. _cfg-aero-cfd-rans_base:

``aero.cfd.rans_base``
^^^^^^^^^^^^^^^^^^^^^^

:型: 文字列
:既定値: ``"model"``
:値/範囲: ``"model"`` ``"cfd"``
:説明: RANS の底面抗力の出どころ。``"model"`` は部品積み上げ法の底面抗力、``"cfd"`` は RANS が平らな底面に積分した力。RANS だけで ``"cfd"`` を指定できる。
:関連: :ref:`sec-cfd-post`、:ref:`IY-C124`

.. _cfg-aero-cfd-iterations:

``aero.cfd.iterations``
^^^^^^^^^^^^^^^^^^^^^^^

:型: 整数
:既定値: ``3000``\ （RANS は ``4000``）
:値/範囲: 1 以上
:説明: 1 ケースの最大反復数。RANS では 2 次精度の段の反復数。
:関連: :ref:`IY-C130`

.. _cfg-aero-cfd-min_iterations:

``aero.cfd.min_iterations``
^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 整数
:既定値: ``100``\ （RANS は ``400``）
:値/範囲: ``stable_window`` 以上
:説明: 早期終了の最小反復数。これより前にはケースを止めない。Euler の Roe の制限関数もこの反復で固定する（RANS の制限関数は固定しない）。
:関連: :ref:`sec-cfd-int-convergence`、:ref:`IY-C133`

.. _cfg-aero-cfd-stable_window:

``aero.cfd.stable_window``
^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 整数
:既定値: ``60``\ （RANS は ``200``）
:値/範囲: 10 以上
:説明: 早期終了の判定で、力とモーメントの係数が落ち着いているかを見る直近の反復数。
:関連: :ref:`sec-cfd-int-convergence`、:ref:`sec-cfd-int-rans-accept`、:ref:`IY-C132`

.. _cfg-aero-cfd-multigrid:

``aero.cfd.multigrid``
^^^^^^^^^^^^^^^^^^^^^^

:型: 整数
:既定値: ``0``
:値/範囲: 0 以上 4 以下
:説明: 凝集型マルチグリッドの粗い格子の数（SU2 の ``MGLEVEL``）。
:関連: :ref:`sec-cfd-multigrid`、:ref:`IY-C131`

.. _cfg-aero-cfd-multigrid_cycle:

``aero.cfd.multigrid_cycle``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 文字列
:既定値: ``"w"``
:値/範囲: ``"w"`` ``"v"``
:説明: マルチグリッドのサイクル。
:関連: :ref:`sec-cfd-multigrid`

.. _cfg-aero-cfd-cfl:

``aero.cfd.cfl``
^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``5.0``
:値/範囲: 正
:説明: CFL 数の初期値（適応的に変える）。
:関連: :ref:`IY-C106`

.. _cfg-aero-cfd-convergence:

``aero.cfd.convergence``
^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``1e-6``
:値/範囲: 0 と 1 の間（両端を含まない）
:説明: Euler の密度の二乗平均残差の目標。RANS は冷えた起動の残差の最大値からの下がりで判定する。
:関連: :ref:`sec-cfd-int-rans-accept`、:ref:`IY-C129`

.. _cfg-aero-cfd-scheme:

``aero.cfd.scheme``
^^^^^^^^^^^^^^^^^^^

:型: 文字列
:既定値: ``"roe"``
:値/範囲: ``"roe"`` ``"jst"``
:説明: 対流フラックスの離散化。
:関連: :ref:`sec-cfd-int-discretization`

.. _cfg-aero-cfd-prefix:

``aero.cfd.prefix``
^^^^^^^^^^^^^^^^^^^

:型: 文字列
:既定値: ``""``
:説明: CFD ツールの環境（``<prefix>/bin/SU2_CFD``、``mpirun``、gmsh を持つ ``python``）。空なら環境変数 ``IGNISYEET_CFD_PREFIX``、それも空なら ``PATH`` を使う。先頭の ``~`` は展開する。
:関連: :ref:`page-howto-cfd-setup`

.. _cfg-aero-cfd-su2:

``aero.cfd.su2``
^^^^^^^^^^^^^^^^

:型: 文字列
:既定値: ``"SU2_CFD"``
:値/範囲: 空でない
:説明: SU2 の実行ファイルの名前またはパス。既定のままのときは、環境変数 ``IGNISYEET_SU2_CFD``、``install.sh`` が構築した SU2、``prefix`` と ``PATH`` の ``SU2_CFD`` の順に使う。明示的に書いたパスが優先される。
:関連: :ref:`page-howto-cfd-setup`、:ref:`IY-C135`

.. _cfg-aero-cfd-mpi:

``aero.cfd.mpi``
^^^^^^^^^^^^^^^^

:型: 文字列
:既定値: ``"mpirun"``
:説明: MPI の起動コマンド。``ranks_per_case`` が 2 以上のときだけ使う。
:関連: :ref:`page-howto-cfd-setup`

.. _cfg-aero-cfd-ranks_per_case:

``aero.cfd.ranks_per_case``
^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 整数
:既定値: ``4``
:値/範囲: 0 以上（0 は自動）
:説明: 1 ケースの MPI ランク数。0 は 4 とスレッド数の小さいほう。
:関連: :ref:`sec-cfd-resources`、:ref:`IY-C510`

.. _cfg-aero-cfd-parallel_cases:

``aero.cfd.parallel_cases``
^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 整数
:既定値: ``4``
:値/範囲: 0 以上（0 は自動）
:説明: 同時に解くケース数。0 はメモリとスレッドの予算に収まる数。``ranks_per_case`` × ``parallel_cases`` が使えるスレッド数を超えると警告する。
:関連: :ref:`sec-cfd-resources`、:ref:`IY-C510`

.. _cfg-aero-cfd-timeout_minutes:

``aero.cfd.timeout_minutes``
^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:型: 数値
:既定値: ``0``
:値/範囲: 0 以上（0 は無制限）
:説明: 1 ケースの時間の上限 [分]。上限に達したケースは、係数が落ち着いていれば受け入れ、そうでなければ失敗とする。
:関連: :ref:`IY-C134`

.. _cfg-aero-cfd-workdir:

``aero.cfd.workdir``
^^^^^^^^^^^^^^^^^^^^

:型: パス
:既定値: ``"cfd"``
:説明: ``output.dir`` の下の作業ディレクトリ。ケースごとのサブディレクトリと中間ファイルを置く。
:関連: :ref:`sec-file-cfd`

