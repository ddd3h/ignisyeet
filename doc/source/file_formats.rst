ファイル形式リファレンス
========================

この章は、IgnisYeet が書き出す、または読み込むファイルの列と項目を、形式ごとにまとめたものである。
どのコマンドがどのファイルを書くかの一覧は :doc:`usage` の「出力ファイル」にあり、各ファイルの値の意味は、係数表なら :doc:`aero_coefficients`、
CFD の出力なら :doc:`cfd` と :doc:`cfd_internals` で述べる。
ここでは、列の並び、単位、必須と任意の別、読み込み時の検査を、実装に合わせて書く。

.. contents:: この章の内容
   :local:
   :depth: 2

.. _sec-aero-table-format:

係数表（``aero_table.csv`` と ``aero_table.json``）
------------------------------------------------------------------

係数表は、CSV 本体と、拡張子を ``.json`` に替えた同名の付随ファイル（メタデータ）の 2 つで 1 組である。
``aero`` が ``out/aero_table.csv`` と ``out/aero_table.json`` を書き、``aero.method = "table"`` では同じ形式の外部の表を読み込む（:ref:`sec-aero-table-format-external`）。
表の種類は付随ファイルの ``kind`` で決まり、``"axisymmetric"``\ （軸対称表。既定）と ``"general"``\ （一般表）がある。
表の概念と、軸対称表と一般表の関係は :ref:`sec-aero-table` と :doc:`aero_coefficients` にある。

CSV の値は、``mach`` ``alpha_deg`` ``beta_deg`` の格子の値を除いて、指数表記で有効数字 10 桁（小数点以下 9 桁）で書かれる。
CSV は、1 行目が見出し、2 行目以降が格子点 1 つにつき 1 行で、空行は読み飛ばす。

.. _sec-file-axisym:

軸対称表の CSV
~~~~~~~~~~~~~~

見出しは次のとおりである。

.. code-block:: text

   mach,alpha_deg,cn,ca_on,ca_off,xcp,cna,damp_s0,damp_s1,damp_s2,cl_delta,clp

行は Mach 数が外側、迎角が内側の順（Mach 数ごとに迎角を小さい方から並べる）に、全格子点を 1 行ずつ並べる。
``alpha_deg`` は全迎角 :math:`\alpha_t` [deg] である（一般表では :math:`\alpha`。:ref:`sec-aero-alpha-beta`）。

.. list-table::
   :header-rows: 1
   :widths: 20 20 60

   * - 列
     - 単位
     - 内容
   * - ``mach``
     - 無次元
     - Mach 数
   * - ``alpha_deg``
     - deg
     - 全迎角 :math:`\alpha_t`
   * - ``cn``
     - 無次元
     - 法線力係数 :math:`C_N`
   * - ``ca_on`` / ``ca_off``
     - 無次元
     - 軸力係数 :math:`C_A`\ （燃焼中 / 燃焼後）
   * - ``xcp``
     - m
     - 圧力中心 :math:`x_{cp}`\ （ノーズ先端から後方へ測る）
   * - ``cna``
     - 1/rad
     - 法線力係数の傾き :math:`C_{N\alpha}`
   * - ``damp_s0`` / ``damp_s1`` / ``damp_s2``
     - 順に m\ :sup:`0`、m\ :sup:`1`、m\ :sup:`2` を含む量
     - ピッチ減衰の和 :math:`S_0, S_1, S_2`\ （:eq:`eq-moment-damp`）
   * - ``cl_delta``
     - 1/rad
     - フィンのカント 1 rad あたりのロールモーメント係数 :math:`C_{l\delta}`
   * - ``clp``
     - 無次元
     - ロール減衰微係数 :math:`C_{lp}=\partial C_l/\partial\hat p`\ （:math:`\hat p=pd/(2V)`。負）

既定の格子は、Mach 数が :math:`M=0,0.02,\dots,3.0`\ （151 点）、全迎角が :math:`\alpha_t=0^\circ,1^\circ,\dots,30^\circ`\ （31 点）である
（設定 ``aero.mach_min`` ``mach_max`` ``mach_step`` と ``alpha_max_deg`` ``alpha_step_deg``。:ref:`[aero] の設定項目 <sec-config-aero>`）。
飛翔計算は 0 から 90° の :math:`\alpha_t'` で表を引き、表の範囲を超える迎角は外挿する（:ref:`sec-aero-table`、:ref:`sec-aero-interp`）。

.. _sec-file-general:

一般表の CSV
~~~~~~~~~~~~

見出しは次のとおりである。

.. code-block:: text

   mach,alpha_deg,beta_deg,cx_on,cx_off,cy,cz,cl,cm,cn,cl_delta,clp,sq0,sq1,sq2,sr0,sr1,sr2

行は Mach 数が最も外側、迎角が次、横滑り角が最も内側の順に、全格子点を 1 行ずつ並べる。
``alpha_deg`` は迎角 :math:`\alpha`、``beta_deg`` は横滑り角 :math:`\beta` である（定義は :ref:`sec-aero-angles`）。
行は 3 つの格子値に続く 15 個の値を、必ず全部持つ。

.. list-table::
   :header-rows: 1
   :widths: 20 20 60

   * - 列
     - 単位
     - 内容
   * - ``mach``, ``alpha_deg``, ``beta_deg``
     - 無次元、deg、deg
     - 格子点の Mach 数、迎角 :math:`\alpha`、横滑り角 :math:`\beta`
   * - ``cx_on`` / ``cx_off``
     - 無次元
     - 機体座標の :math:`x_b` 方向の力係数 :math:`C_X`\ （燃焼中 / 燃焼後）
   * - ``cy`` / ``cz``
     - 無次元
     - :math:`y_b` 方向、:math:`z_b` 方向の力係数 :math:`C_Y`, :math:`C_Z`
   * - ``cl`` / ``cm`` / ``cn``
     - 無次元
     - ノーズ先端まわりのモーメント係数 :math:`C_l`, :math:`C_m`, :math:`C_n`\ （:math:`qSd` で無次元化。``cn`` は 6 成分のヨーモーメントで、軸対称表の法線力係数 :math:`C_N` ではない。:ref:`sec-aero-cn-cz`）
   * - ``cl_delta``
     - 1/rad
     - カント 1 rad あたりのロールモーメント係数 :math:`C_{l\delta}`\ （表のカントの基準 ``fin_cant_ref_deg`` との差の分だけ飛翔計算が足す）
   * - ``clp``
     - 無次元
     - ロール減衰微係数 :math:`C_{lp}`
   * - ``sq0`` / ``sq1`` / ``sq2``
     - ``damp_s0`` から ``damp_s2`` と同じ
     - ピッチの減衰の和 :math:`S_0^q,S_1^q,S_2^q`
   * - ``sr0`` / ``sr1`` / ``sr2``
     - ``damp_s0`` から ``damp_s2`` と同じ
     - ヨーの減衰の和 :math:`S_0^r,S_1^r,S_2^r`

一般表では、単一の圧力中心 :math:`x_{cp}` や :math:`C_{N\alpha}` を飛翔計算に使わない。
6 成分の定義、符号、基準点は :doc:`aero_coefficients` と :ref:`sec-aero-forces`、減衰の和の使い方は :eq:`eq-moment-damp` にある。

迎角 :math:`-180^\circ\le\alpha\le180^\circ` と横滑り角 :math:`-90^\circ\le\beta\le90^\circ` の格子は不等間隔である。
:math:`\pm30^\circ` の内側は 2° 刻み、外側は 10° 刻みで、迎角は 61 点、横滑り角は 43 点になる。
軸対称表から展開した一般表（部品積み上げ法、パネル法、外部の軸対称表）は、この格子を使う。CFD の一般表は同じ格子に値を埋めて作る（:ref:`sec-cfd-general`）。
外部の一般表の格子は不等間隔でもよく、付随ファイルの ``alphas_deg`` と ``betas_deg`` が決める。

Mach 数の刻みは、作り方で異なる。

* 部品積み上げ法：``aero.general_mach_step``\ （既定 0.05。``method = "barrowman"`` かつ ``table_kind = "general"`` のときだけ効く）
* パネル法と外部の軸対称表：その軸対称表の Mach 数の格子
* CFD：``aero.cfd.general_mach_step``\ （既定 0.1。0 のとき ``aero.mach_step``）

サンプルの機体の一般表（部品積み上げ法）は、61（Mach 数）× 61（迎角）× 43（横滑り角）= 160003 行になる。
軸対称表から展開した一般表は、軸対称表より情報を増やさない（飛翔の比較は :ref:`sec-vaero-table` に示す）。

.. _sec-file-sidecar:

付随ファイル ``aero_table.json``
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

付随ファイルは、表の基準量、格子、種類、入力のハッシュ値を持つ JSON である。書き出すときは、次の項目をこの順に書く。

.. list-table::
   :header-rows: 1
   :widths: 24 14 62
   :class: longtable

   * - 項目
     - 読み込み
     - 内容
   * - ``format_version``
     - 任意
     - 形式のバージョン。書き出すときは常に 2。読み込み時はこの値を使わず、CSV の列数で形式を判定する（:ref:`sec-file-v1`）
   * - ``kind``
     - 任意
     - ``"axisymmetric"``\ （既定）または ``"general"``
   * - ``source_hash``
     - 必須
     - 入力のハッシュ値。``aero`` は STL の内容、設定値、プログラムのバージョンから計算した FNV-1a の値（16 桁の 16 進数）を書き、保存済みの表を使うかの判定に使う。外部の表では任意の文字列でよい
   * - ``ref_area``
     - 必須
     - 基準面積 [m\ :sup:`2`]
   * - ``ref_diameter``
     - 必須
     - 基準直径 :math:`d` [m]
   * - ``length``
     - 必須
     - 全長 [m]。``motor.aft_x`` を省略したときのモータ後端位置はこの値になる
   * - ``machs``
     - 必須
     - Mach 数の格子（CSV の ``mach`` 列の値を昇順に）
   * - ``alphas_deg``
     - 必須
     - 迎角の格子 [deg]。軸対称表では全迎角 :math:`\alpha_t`\ （0 以上）、一般表では :math:`\alpha`
   * - ``betas_deg``
     - 任意
     - 一般表の横滑り角の格子 [deg]（:math:`-90^\circ` から :math:`90^\circ`）。軸対称表では空
   * - ``fin_cant_ref_deg``
     - 任意
     - 表の :math:`C_l` に含まれるフィンのカント :math:`\delta_{\mathrm{ref}}` [deg]。既定は 0（:ref:`sec-aero-table`）
   * - ``moment_ref``
     - 任意
     - モーメントの基準点。現在は ``"nose_tip"``\ （既定）だけである（:ref:`sec-aero-moment-ref`）
   * - ``extrapolation``
     - 必須
     - 表の範囲外の扱い。``"linear"``\ （線形外挿）または ``"clamp"``\ （端の値で一定）

部品積み上げ法、パネル法、CFD の軸対称表の ``fin_cant_ref_deg`` は、設定したカントである。CFD の一般表では、``stl`` のとき設定したカント、``panel_mesh`` のとき 0 である（形状にカントがないため）。CFD の表の値は ``cfd_report.json`` の ``fin_cant_ref_deg`` にも出る。

読み込み時の検査
~~~~~~~~~~~~~~~~

CSV の行数が、``machs`` と ``alphas_deg``\ （一般表では ``betas_deg`` も）の積に一致しないと、``aero table size does not match its metadata`` のエラーになる。
値が数として読めない行、列数が合わない行もエラーである。
一般表の行は 18 個の値を全部持たなければならない。軸対称表の行は、12 列の形式か、後述のバージョン 1 の 10 列（格子の 2 列に値 8 個）の形式である。

.. _sec-file-v1:

バージョン 1 の軸対称表
~~~~~~~~~~~~~~~~~~~~~~~

軸対称表の行が値を 8 個（``mach`` ``alpha_deg`` に続けて ``cn`` から ``damp_s2`` まで、合わせて 10 列）しか持たない表は、ロール項 ``cl_delta`` と ``clp`` を 0 とした軸対称表として読み込む。
このとき付随ファイルに ``format_version`` がなくてもよい。保存するときは常にバージョン 2（12 列）で書く。

``aero.method = "table"`` で読み込んだ表のロール項がすべて 0 のとき（バージョン 1 の表を含む）は、``aero``、``sim``、``dispersion`` のどれでも、現在の STL のフィンから部品積み上げ法のロール項（:math:`C_{l\delta}` と :math:`C_{lp}`）を求めて表に補う。
補ったときは、端末に注意書きを表示する。形状を抽出できないときは、注意書きを表示してロール項を 0 のままにし、機体はロールしない。フィンがなければ補う値も 0 で、注意書きは出ない。
ロール項を含む表は、そのまま使う。補った値は、読み込んだ表のメモリ上だけに入り、元のファイルは書き換えない。

.. _sec-aero-table-format-external:

外部の表を作るときの要点
~~~~~~~~~~~~~~~~~~~~~~~~

外部の表（``aero.method = "table"``）は、上の形式のとおりに CSV と付随ファイルを用意する。使い方は :ref:`sec-usage-external-table` にある。確認する点は次のとおりである。

* 付随ファイルは CSV と同じ名前で拡張子が ``.json`` のファイルで、必須の項目（``source_hash`` ``ref_area`` ``ref_diameter`` ``length`` ``machs`` ``alphas_deg`` ``extrapolation``）をすべて書く。
* 軸対称表の ``alpha_deg`` は全迎角 :math:`\alpha_t`、一般表の ``alpha_deg`` は迎角 :math:`\alpha`、``beta_deg`` は横滑り角 :math:`\beta` である。軸対称表の ``cn`` は法線力係数、一般表の ``cn`` はヨーモーメント係数 :math:`C_n` で、意味が違う（:ref:`sec-aero-cn-cz`）。
* モーメントはノーズ先端まわり（``moment_ref = "nose_tip"``）で、:math:`qSd` で無次元化する。
* 一般表は ``kind = "general"`` とし、``betas_deg`` を書く。格子は不等間隔でもよい。
* 軸対称な外部の表は、``aero.table_kind = "general"`` で一般表に展開できる（展開した表は書き出さない）。外部の表が最初から一般表なら、そのまま使う。
* ロール項がすべて 0 の表には、前節のとおり部品積み上げ法のロール項が補われる。

.. _sec-file-drag:

``aero_drag.csv``
-----------------

Mach 数ごとの抗力の内訳である。``method = "barrowman"`` と ``"panel"`` の ``aero`` が書く（``"cfd"`` は書かない）。

部品積み上げ法（``method = "barrowman"``）の見出しは次のとおりである。

.. code-block:: text

   mach,reynolds,friction,nose,base,fins,boattail,extra,cd_off,cd_on,cna,xcp

``reynolds`` は Reynolds 数、``friction`` ``nose`` ``base`` ``fins`` ``boattail`` ``extra`` は燃焼後の抗力係数の内訳（摩擦、ノーズ、底面、フィン、ボートテール、``extra_cd``）、
``cd_off`` と ``cd_on`` は燃焼後と燃焼中の合計である。``cna``\ （1/rad）と ``xcp``\ （m、ノーズ先端から）は :math:`\alpha=0` での値である。

``method = "panel"`` では内訳を持たず、見出しは次のとおりである。

.. code-block:: text

   mach,cd_off,cd_on,cna,xcp

``cd_off`` と ``cd_on`` は、表の :math:`\alpha=0` の軸力係数 ``ca_off`` と ``ca_on`` である。

.. _sec-file-panel-cp:

``panel_cp.csv``
----------------

``method = "panel"`` が書く、表面パネルの圧力係数である。見出しは次のとおりである。

.. code-block:: text

   x,y,z,nx,ny,nz,area,cp,part

``x`` ``y`` ``z`` はパネルの位置 [m]、``nx`` ``ny`` ``nz`` は法線、``area`` は面積 [m\ :sup:`2`]、``cp`` は圧力係数、``part`` は部位（``body`` または ``fin``）である。
値は迎角 4°、最も低い亜音速 Mach 数（既定は :math:`M=0`）の解である。``python/plot.py panel`` で作図する（:doc:`panel`）。

.. _sec-file-cfd:

CFD の出力
----------

``method = "cfd"`` は、係数表に加えて、次のファイルを書く（``aero_drag.csv`` は書かない）。値の読み方は :doc:`cfd` にある。

``cfd_cases.csv``
~~~~~~~~~~~~~~~~~

解いたケースを 1 行に 1 つずつ書く。見出しは次のとおりである。

.. code-block:: text

   mach,alpha_deg,status,iterations,residual_first,residual_last,cn_raw,ca_cfd,nose_moment_raw,xcp_raw_m,cn,nose_moment,xcp_m,wall_seconds,warm_start,beta_deg,roll_hat,cx,cy,cz,cl,cm,cn_yaw

.. list-table::
   :header-rows: 1
   :widths: 26 74
   :class: longtable

   * - 列
     - 内容
   * - ``mach`` ``alpha_deg`` ``beta_deg``
     - ケースの Mach 数、迎角 [deg]、横滑り角 [deg]
   * - ``status``
     - ``converged``\ （収束）、``accepted``\ （収束の判定を満たさないまま、係数が落ち着いているとして受け入れた）、``failed``\ （失敗）。``alpha_mode = "mirror"`` の軸対称表では、正負の解を合わせた行が ``combined`` で末尾に付く
   * - ``iterations``
     - 反復数（失敗したケースは 0）
   * - ``residual_first`` / ``residual_last``
     - 最初と最後の反復の密度残差（対数）
   * - ``cn_raw`` / ``ca_cfd``
     - SU2 の生の法線力係数 :math:`C_N` と軸力係数 :math:`C_A`
   * - ``nose_moment_raw`` / ``xcp_raw_m``
     - SU2 の生のノーズ先端まわりのモーメント（長さの次元 [m]。:math:`C_m` に相当する量）と、そこから求めた圧力中心 [m]
   * - ``cn`` / ``nose_moment`` / ``xcp_m``
     - 表が使う値。``offset`` では :math:`\alpha=0` の値を引いた値、``single`` では生の値と同じ、``mirror`` では生の行は空で ``combined`` の行にだけ入る。一般表のケースでは空
   * - ``wall_seconds``
     - 1 ケースの計算時間 [s]
   * - ``warm_start``
     - 暖機起動（別のケースの解から始めたか）の有無（``true`` または ``false``）
   * - ``roll_hat``
     - ロール減衰のケースの無次元ロール角速度 :math:`\hat p`\ （それ以外は 0）
   * - ``cx`` ``cy`` ``cz`` ``cl`` ``cm`` ``cn_yaw``
     - 機体軸の 6 成分 :math:`C_X,C_Y,C_Z,C_l,C_m,C_n`\ （``cn_yaw`` が :math:`C_n`。法線力係数の ``cn`` と区別する）。結果のないケースは空

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
     - RANS のとき、壁面の最初のセルの :math:`y^+` の見積もり
   * - ``table_kind``
     - 係数表の種類（``axisymmetric`` または ``general``）
   * - ``shape_symmetry`` / ``domain``
     - 実際に使った形状の対称性（``axisymmetric``、``pitch_plane``、``none``）と計算領域（``half`` または ``full``）
   * - ``betas_deg``
     - 解いた横滑り角
   * - ``cfd_region`` / ``model_outside_cfd_region``
     - 一般表で、CFD の値を使う範囲（``alpha_min_deg`` ``alpha_max_deg``
       ``beta_min_deg`` ``beta_max_deg``）と、その外側がモデルの値かどうか（一般表では ``true``）
   * - ``clp`` / ``clp_source``
     - Mach 数ごとのロール減衰の CFD の値、部品積み上げ法の値、採用した側と理由（``clp``）と、全体の採用元（``cfd``、``barrowman``、``mixed``）
   * - ``diagnostics``
     - Mach 数ごとの零角度の傾き（``cna_pitch`` ``xcp_pitch`` ``cna_yaw`` ``xcp_yaw``）。一般表の入力ではなく診断用で、軸対称表ではヨーの値は入らない
   * - ``yaw_damping_from_pitch``
     - 横滑り角のデータのない一般表で、ヨー減衰の和がピッチと同じかどうか
   * - ``fin_cant_ref_deg``
     - CFD の形状に含まれるフィンのカント [deg]（ロールの強制項の基準）
   * - ``base_treatment``
     - 底面の扱い（``fairing``、``flat``）。壁面が STL のときと Euler のときに入る
   * - ``warnings``
     - 警告の一覧
   * - ``symmetry_check``
     - ``surface = "stl"`` の鏡映対称性の判定。``requested``\ （設定値）、``chosen``\ （採用した対称性）、平面ごとの検査 ``y`` と ``z``\ （``symmetric``、``max_mismatch`` [m]、``vertices``、``vertices_matched``、``triangulation_identical``、``tolerance``、``reason``）、メッシュの作り方 ``meshed``\ （``quarter``、``half``、``full``）、``reason``
   * - ``fidelity``
     - ``surface = "stl"`` の形状の忠実度。``fidelity``\ （``stl_fidelity.json`` と同じ内容）、``union``\ （``union_report.json``）、``repair``\ （``repair_report.json``）を持つ

.. _sec-file-mesh:

``<workdir>/mesh/`` のファイル
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

CFD のメッシュのキャッシュで、``mesh.su2`` などが入る。``surface = "stl"`` では、次のファイルも入る。

``repair_report.json``
   STL の修復の報告。``input_triangles``\ （入力の三角形数）、``weld_tolerance``\ （頂点を溶接する許容値）、``vertices``\ （溶接後の頂点数）、
   ``degenerate_dropped`` と ``duplicate_dropped``\ （捨てた退化した三角形と重複した三角形の数）、``components``\ （閉じた殻ごとの ``triangles``、``volume`` [m\ :sup:`3`]、
   ``flipped``\ （内外が逆で全面を反転したか）、``reoriented_facets``\ （向きをそろえるために反転した面の数）、``bbox_min``、``bbox_max``）を持つ。
   修復は位相だけを直し、溶接以外で頂点を動かさない。
   修復した殻の STL は ``component_0.stl`` などに書かれる。

``union.stl`` と ``union_report.json``
   重なった殻の和（manifold3d のブーリアン和）と、その報告。``components``\ （殻ごとの ``index``、``status``、``volume``、``triangles``）、
   ``volume``、``surface_area``、``genus``、``triangles``、``vertices``、``volume_sum_of_components``、``overlap_volume``、``bbox_min``、``bbox_max`` を持つ。

``stl_fidelity.json``
   形状の忠実度（:ref:`sec-cfd-fidelity`）。``mesh_hash`` とその ``fidelity`` を持ち、``fidelity`` は次の項目を持つ。

   * ``input`` / ``union`` / ``cfd``：入力の STL、和、CFD の壁面それぞれの ``triangles``、``area``、``volume``\ （入力は殻が重なるので ``null``。半領域をそのまま解くときは和と CFD の壁面も ``null``）、``bbox_min``、``bbox_max``
   * ``cfd_wall_triangles`` / ``cfd_base_triangles``：CFD の壁面と底面の三角形数
   * ``cfd_vs_union`` / ``cfd_wall_vs_input``：距離の統計 [m]（``max``、その位置 ``max_at``、``p99``、``rms``、標本数 ``samples``）
   * ``wall_size`` / ``warn_above`` / ``fail_above``：壁面の要素の大きさ、警告の水準（0.5 ``wall_size``）、エラーの水準（``max_geometry_error``）[m]
   * ``genus`` / ``warnings`` / ``symmetry``：和の種数、警告、鏡映対称性の判定（``cfd_report.json`` の ``symmetry_check`` と同じ）

形状と断面のファイル
--------------------

.. _sec-file-geometry:

``geometry.json``
~~~~~~~~~~~~~~~~~

``geom`` と、係数表を作る ``aero`` が書く、抽出した形状である。
全長 ``length``、基準半径 ``ref_radius`` と基準面積 ``ref_area``、底面半径 ``base_radius``、ノーズ ``nose``、ボートテール ``boattail``、胴体のぬれ面積 ``wetted_area_body`` と体積 ``volume``、
フィンの平面形の面積 ``planform_area`` と重心位置 ``planform_centroid``、フィン ``fins``、STL の座標との対応 ``frame``\ （``axis_point``、``aft_direction``、``nose_offset``）、断面ごとの ``profile`` を持つ。
機体軸 :math:`x_b,y_b,z_b` を STL の座標で表した単位ベクトル ``body_x``、``body_y``、``body_z`` も書く（:ref:`sec-body-basis`）。

``profile.csv``
~~~~~~~~~~~~~~~

見出しは ``x,r,fin_span`` で、断面の位置 ``x``、胴体半径 ``r``、フィン高さ ``fin_span`` [m] を断面ごとに書く。

飛翔と落下分散の出力
--------------------

飛翔と落下分散の出力は :doc:`usage` の「出力ファイル」で概要を述べる。ここでは列を示す。

.. _sec-file-trajectory:

``trajectory.csv``
~~~~~~~~~~~~~~~~~~

見出しは次のとおりである。

.. code-block:: text

   t,phase,east,north,up,lat,lon,alt,vel_e,vel_n,vel_u,airspeed,mach,alpha_deg,thrust,mass,x_cg,x_cp,stability_cal,dyn_pressure,cn,ca,pitch_deg,heading_deg,wx,wy,wz,beta_deg,roll_rate,cl,cy

時刻歴で、位置（``east`` ``north`` ``up``、``lat`` ``lon`` ``alt``）、速度、``airspeed``、``mach``、``thrust``、``mass``、``x_cg``、``x_cp``、``stability_cal``\ （静安定余裕）、``dyn_pressure``、姿勢と角速度を持つ。
``phase`` は ``rail``、``free``、``parachute`` のいずれかである。
``alpha_deg`` は全迎角 :math:`\alpha_t`、``x_cp`` は一般表では等価な圧力中心（:eq:`eq-xcp-eff`。表示用の値で、非対称な機体の安定性の尺度ではない）である。
末尾の 4 列は、``beta_deg``\ （横滑り角 :math:`\beta` [deg]）、``roll_rate``\ （ロール角速度 :math:`\omega_x` [rad/s]）、
``cl``\ （ロールの係数 :math:`C_l^{\mathrm{tot}}`、:eq:`eq-cl-total`）、``cy``\ （一般表の :math:`C_Y`。軸対称表では 0）である。

``dispersion.csv``
~~~~~~~~~~~~~~~~~~

``mode = "wind_grid"`` が書く。見出しは次のとおりである。

.. code-block:: text

   descent,wind_speed,wind_direction_deg,landing_east,landing_north,landing_lat,landing_lon,landing_distance,landing_time,landing_speed,apogee,apogee_time,max_mach,rail_exit_speed,min_stability_cal

風の各ケースの着地点（ENU と緯度経度）、頂点、飛行時間などである。

``dispersion_mc.csv``
~~~~~~~~~~~~~~~~~~~~~

``mode = "monte_carlo"`` が書く、サンプルごと・降下モードごとの 1 行である。見出しは次のとおりである。

.. code-block:: text

   sample,descent,status,thrust_scale,burn_time_scale,dry_mass,cg,cn_scale,ca_scale,elevation_deg,azimuth_deg,wind_speed_delta,wind_direction_deg_delta,parachute_cd_s_scale,wind_speed,wind_direction_deg,landing_east,landing_north,landing_lat,landing_lon,landing_distance,landing_time,landing_speed,apogee,apogee_time,max_mach,rail_exit_speed,min_stability_cal,thrust_misalignment_y_deg,thrust_misalignment_z_deg,thrust_offset_y,thrust_offset_z,fin_cant_deg,cg_lateral_y,cg_lateral_z,cl_scale,clp_scale

``status`` は ``ok`` または ``failed`` で、失敗した行は結果の列が空欄になる。
``thrust_scale`` から ``parachute_cd_s_scale`` までは引いたばらつきで、相対のものは割合、``wind_speed_delta`` と ``wind_direction_deg_delta`` は風速・風向のずれである。
``wind_speed`` と ``wind_direction_deg`` はそのサンプルの基準風速・風向、続く列は着地点（ENU と緯度経度）、着地距離、着地時刻、着地速度、頂点、頂点時刻、最大 Mach 数、ランチャ離脱速度、最小静安定余裕である。
末尾の 9 列（``thrust_misalignment_y_deg`` から ``clp_scale``）は、ロールと非対称について引いた値である。

``dispersion_summary.json``
~~~~~~~~~~~~~~~~~~~~~~~~~~~

``mode = "monte_carlo"`` が書く。降下モードごとの有効サンプル数 ``n``、失敗数 ``failed``、着地点の平均と共分散行列、
:math:`1\sigma` と :math:`3\sigma` の誤差楕円（長半径、短半径、長軸の方位）、最大着地距離とそのサンプル番号、頂点高度の平均と標準偏差を持つ（:ref:`sec-mc`）。
