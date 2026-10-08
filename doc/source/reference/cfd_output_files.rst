.. _page-reference-cfd-output-files:

CFD の出力ファイル
====================

``aero.method = "cfd"`` が出力先に書くファイルの列と項目を示す。値の意味は :ref:`page-theory-cfd-choose` と :ref:`page-theory-cfd-coefficients` にある。

.. _sec-file-cfd:

CFD の結果ファイル
------------------

``method = "cfd"`` は、係数表に加えて、次のファイルを書く（``aero_drag.csv`` は書かない）。値の読み方は :ref:`page-theory-cfd-choose` にある。

``cfd_cases.csv``
~~~~~~~~~~~~~~~~~

解いたケースを 1 行に 1 つずつ書く。見出しは次のとおりである。

.. code-block:: text

   mach,alpha_deg,status,iterations,residual_first,residual_last,cn_raw,ca_cfd,nose_moment_raw,xcp_raw_m,cn,nose_moment,xcp_m,wall_seconds,warm_start,beta_deg,roll_hat,cx,cy,cz,cl,cm,cn_yaw,residual_reference,residual_drop,rans_verdict

.. list-table::
   :header-rows: 1
   :widths: 26 74
   :class: longtable

   * - 列
     - 内容
   * - ``mach`` ``alpha_deg`` ``beta_deg``
     - ケースの Mach 数、迎角 [deg]、横滑り角 [deg]
   * - ``status``
     - ``converged``\ （収束）、``accepted``\ （Euler で、収束の判定を満たさないまま、係数が落ち着いているとして受け入れた）、``unconverged``\ （RANS で、係数の振動か残差の停滞のまま、窓の平均を受け入れた。:ref:`sec-cfd-int-rans-accept`）、``failed``\ （失敗）。``alpha_mode = "mirror"`` の軸対称表では、正負の解を合わせた行が ``combined`` で末尾に付く
   * - ``iterations``
     - 反復数（失敗したケースは 0。RANS は 2 次精度の段の反復数）
   * - ``residual_first`` / ``residual_last``
     - 履歴の最初の行と、判定した行の密度残差（対数）。判定した行は、収束で打ち切ったときはその時点の行、打ち切らなかったときは最後の行である。RANS では 2 次精度の段の履歴の値である
   * - ``cn_raw`` / ``ca_cfd``
     - SU2 の生の法線力係数 :math:`C_N` と軸力係数 :math:`C_A`\ （RANS の :math:`C_A` は壁面の ``CFx(wall)``、``rans_base = "cfd"`` では底面の ``CFx(base)`` を足した値。:ref:`sec-cfd-post`）
   * - ``nose_moment_raw`` / ``xcp_raw_m``
     - SU2 の生のノーズ先端まわりのモーメント（長さの次元 [m]。:math:`C_m` に相当する量）と、そこから求めた圧力中心 [m]
   * - ``cn`` / ``nose_moment`` / ``xcp_m``
     - 表が使う値。``offset`` では :math:`\alpha=0` の値を引いた値、``single`` では生の値と同じ、``mirror`` では生の行は空で ``combined`` の行にだけ入る。一般表のケースでは空
   * - ``wall_seconds``
     - 1 ケースの計算時間 [s]
   * - ``warm_start``
     - 別のケースの解から始めたかどうか（``true`` または ``false``）
   * - ``roll_hat``
     - ロール減衰のケースの無次元ロール角速度 :math:`\hat p`\ （それ以外は 0）
   * - ``cx`` ``cy`` ``cz`` ``cl`` ``cm`` ``cn_yaw``
     - 機体軸の 6 成分 :math:`C_X,C_Y,C_Z,C_l,C_m,C_n`\ （``cn_yaw`` が :math:`C_n`。法線力係数の ``cn`` と区別する）。結果のないケースは空
   * - ``residual_reference`` / ``residual_drop`` / ``rans_verdict``
     - RANS の受け入れの記録（:ref:`sec-cfd-int-rans-accept`）。``residual_reference`` は基準の最大値（密度の残差。対数）、``residual_drop`` は判定の根拠にした密度の残差の下がり [桁]、``rans_verdict`` は ``converged``、``limit_cycle``、``stalled`` のいずれか。Euler のケース、失敗したケース、``combined`` の行では空

``cfd_report.json``
~~~~~~~~~~~~~~~~~~~

CFD の実行の要約である。項目は次のとおりである。

.. list-table::
   :header-rows: 1
   :widths: 45 55
   :class: longtable

   * - 項目
     - 内容
   * - ``cases``
     - ケース数
   * - ``failed`` / ``unconverged``
     - 失敗したケース、未収束のケースの ``[Mach 数, 迎角]`` の一覧
   * - ``failed_cases``
     - 失敗したケースのディレクトリ
   * - ``unconverged_cases``
     - 未収束のケースのディレクトリ
   * - ``stalled_cases``
     - RANS で残差が停滞したまま受け入れたケース。``case``\ （ディレクトリ名）と ``residual_drop``\ （密度の残差の下がり [桁]）の組の一覧
   * - ``filled``
     - 失敗して補間で埋めたケースの一覧
   * - ``wall_seconds`` / ``case_seconds``
     - 全体の計算時間 [s]、各ケースの計算時間の合計 [s]（ケースは同時に走るので、合計のほうが長い）
   * - ``mesh_nodes`` / ``mesh_cells``
     - メッシュの節点数とセル数
   * - ``alpha_mode``
     - ``mirror``、``offset``、``single``
   * - ``asymmetry``
     - 軸対称表で、測定した :math:`\alpha=0` の非対称性（Mach 数ごとの ``cn0`` と ``mom0``）
   * - ``ranks_per_case`` / ``parallel_cases`` / ``bytes_per_case``
     - 実際に使った 1 ケースあたりの MPI ランク数、同時に解いたケース数、1 ケースのメモリの見積もり [byte]
   * - ``est_wall_yplus``
     - RANS のとき、最初の層の高さから見積もった、機体の全長の位置での :math:`y^+`\ （:ref:`sec-cfd-prism`）
   * - ``table_kind``
     - 係数表の種類（``axisymmetric`` または ``general``）
   * - ``shape_symmetry`` / ``domain``
     - 実際に使った形状の対称性（``axisymmetric``、``pitch_plane``、``none``）と計算領域（``half`` または ``full``）
   * - ``betas_deg``
     - 解いた横滑り角
   * - ``cfd_region`` / ``model_outside_cfd_region``
     - 一般表で、CFD の値を使う範囲（``alpha_min_deg`` ``alpha_max_deg``
       ``beta_min_deg`` ``beta_max_deg``）と、その外側がモデルの値かどうか（一般表では ``true``）
   * - ``mach_clamped_above`` / ``mach_clamped_below``
     - 表が端の値で一定になる境界の Mach 数。表の Mach 数の格子が解いた最大（最小）の Mach 数より上（下）まで広いときだけ入り、値は解いた最大（最小）の Mach 数である。CFD の表は Mach 数の方向に外挿しない
   * - ``clp`` / ``clp_source``
     - Mach 数ごとのロール減衰の CFD の値、部品積み上げ法の値、採用した側と理由（``clp``）と、全体の採用元（``cfd``、``barrowman``、``mixed``）
   * - ``diagnostics``
     - Mach 数ごとの零角度の傾き（``cna_pitch`` ``xcp_pitch`` ``cna_yaw`` ``xcp_yaw``）。一般表の入力ではなく診断用で、軸対称表ではヨーの値は入らない
   * - ``yaw_damping_from_pitch``
     - 横滑り角のデータのない一般表で、ヨー減衰の和がピッチと同じかどうか
   * - ``fin_cant_ref_deg``
     - CFD の形状に含まれるフィンのカント [deg]（ロールの強制項の基準）
   * - ``base_treatment``
     - 底面の扱い（``fairing``、``flat``）。壁面が STL か CAD のモデルのときと、Euler のときに入る。``surface = "step"`` の RANS では ``none``
   * - ``warnings``
     - 警告の一覧。各文字列の先頭にコード（``[IY-F5nn] …`` の形）が付く
   * - ``symmetry_check``
     - ``surface = "stl"`` と ``"step"`` の鏡映対称性の判定。``requested``\ （設定値）、``chosen``\ （採用した対称性）、平面ごとの検査 ``y`` と ``z``\ （``symmetric``、``max_mismatch`` [m]、``vertices``、``vertices_matched``、``triangulation_identical``、``tolerance``、``reason``）、メッシュの作り方 ``meshed``\ （``quarter``、``half``、``full``）、``reason``。
       ``step`` では、``vertices`` は鏡映した曲面上の点の数、``vertices_matched`` は曲面に一致した点の数、``tolerance`` は実際に使った許容（:ref:`sec-cfd-step-internals`）で、``triangulation_identical`` は常に ``false`` である
   * - ``fidelity``
     - ``surface = "stl"`` の形状の忠実度。``fidelity``\ （``stl_fidelity.json`` と同じ内容）、``union``\ （``union_report.json``）、``repair``\ （``repair_report.json``）を持つ
   * - ``step``
     - ``surface = "step"`` のときだけ入る。CAD のモデルの読み込み、位置合わせ、計算領域（:ref:`sec-cfd-step`）。
       ``file``\ （CAD のファイル）、``format``\ （``step``、``iges``、``brep``）、``file_unit_m``\ （ファイルが宣言する単位 [m]、宣言がなければ ``null``）、``step_unit`` と ``unit_scale``\ （設定した単位とそこから決まる倍率）、
       ``solids_in_file`` と ``solids``\ （融合の前後のソリッドの数）、``faces`` と ``base_faces``\ （面の数と底面の面の数）、``volume`` [m\ :sup:`3`]、``area`` [m\ :sup:`2`]、``import_notes``\ （縫い合わせ、修復、融合の注記）、
       ``registration``\ （位置合わせ。下記）、``symmetry``\ （``symmetry_check`` と同じ）、``domain``\ （``full``、``half``\ （SU2 の半領域）、``half_mirrored``\ （半領域を鏡映した全領域）、``quarter_half``、``quarter_full``）、
       ``base_treatment``\ （``fairing``、``flat``、``none``）、``fairing``\ （円すいを付けた底面の位置 ``x``、半径 ``radius``、長さ ``length``、切った位置 ``cut_at``、底面の面積 ``base_face_area``）、
       ``smallest_feature`` と ``min_size``\ （最も小さい面の幅と最小の要素の大きさ [m]）、``warnings`` を持つ。

       ``registration`` は、``threshold``\ （STL と CAD の面の距離の許容 [m]）、``detail_threshold``\ （CAD の細部とみなす距離 [m]）、
       ``stl_to_step`` と ``step_to_stl``\ （距離の統計 [m]。``max``、その位置 ``max_at``、``p99``、``rms``、標本数 ``samples``）、``stl_buried_fraction``\ （CAD の立体に埋まって除いた STL の点の割合）、
       ``step_only_regions`` と ``step_only_area``\ （CAD だけにある細部の領域の数と面積 [m\ :sup:`2`]）、``stl_bbox``、``step_bbox``、``bbox_offsets``\ （包含箱 ``[xmin, ymin, zmin, xmax, ymax, zmax]`` と CAD − STL の差 [m]）、
       ``problems``\ （合わない理由。通ったときは空）、``hint``\ （推定した原因）を持つ
   * - ``axial_breakdown``
     - RANS のとき、ロール減衰でないケースごとの軸力の内訳。``mach``、``alpha_deg``、``cfd_wall_ca`` と ``cfd_base_ca``\ （壁面と底面の ``CFx``。履歴に列がなければ ``null``）、
       ``barrowman_friction`` と ``barrowman_base``\ （その Mach 数の部品積み上げ法の摩擦と底面の抗力。比べるための値で、摩擦は表に足さない）、
       ``roughness_increment``\ （表に足す、設定の粗さによる摩擦の増分。:eq:`eq-cfd-rans-rough`）、``fin_edge_increment``\ （表に足す、設定のフィンの縁の増分。:eq:`eq-cfd-rans-fin`。``stl`` と ``step`` では 0）、``fin_te_base_increment``\ （表に足す、底面にそろったフィンの後縁の底面抗力。:eq:`eq-cfd-te-flush`。``stl`` と ``step`` で底面をモデルから取るときだけ。それ以外は 0）、``base_from_cfd``\ （表の底面の項が CFD の値か）を持つ。Euler では空


.. _sec-file-mesh:

``<workdir>/`` のファイル
-------------------------

``output.dir`` の下の ``aero.cfd.workdir`` に、計算の作業ディレクトリが置かれる。次のファイルが入る。

.. list-table::
   :header-rows: 1
   :widths: 34 66

   * - パス
     - 内容
   * - ``m0.800_a04.00/`` のようなケースごとのディレクトリ
     - SU2 の設定、履歴、解、そのケースの結果 ``done.json``。ケースの条件（Mach 数、迎角、横滑り角）はディレクトリ名に入る
   * - ``mesh/mesh.su2``
     - 体積メッシュ
   * - ``mesh/mesh_stats.json``
     - メッシュの節点数、セル数、セルの種類ごとの数、品質（SICN）の統計、壁面の三角形数。RANS のメッシュはプリズム層の数、最初の層の高さ、厚さも持つ
   * - ``mesh/stl_fidelity.json`` ``mesh/union_report.json`` ``mesh/repair_report.json``
     - ``surface = "stl"`` のとき。形状の忠実度、重なった殻の和、STL の修復の報告。内容は ``cfd_report.json`` の ``fidelity`` に入る
   * - ``mesh/step_report.json``
     - ``surface = "step"`` のとき。CAD のモデルの読み込みと位置合わせの報告。内容は ``cfd_report.json`` の ``step`` に入る

同じ入力のケースとメッシュは、次の実行で再利用される。途中で止めた計算や、設定の一部だけを変えた計算は、影響を受けないケースから先を計算する。
``ignisyeet cfd-check 設定ファイル --mesh`` は、``surface = "step"`` のとき ``CAD surface (surface = "step", base treatment: …):`` で始まる報告を表示する。
行は ``file``、``shape``、``details``、``domain``、``base``、``registration`` 以下の位置合わせの 4 行で、``note:`` と ``warning:`` の行が続くことがある。値は ``cfd_report.json`` の ``step`` と同じである（:ref:`sec-cfd-step-report`）。
