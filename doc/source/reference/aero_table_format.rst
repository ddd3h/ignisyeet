.. _page-reference-aero-table-format:

係数表の形式
==============

``aero_table.csv``\ （と付随ファイル ``aero_table.json``）、``aero_drag.csv``、``panel_cp.csv`` の列と項目を、形式ごとにまとめる。
外部の係数表（``aero.method = "table"``）は、ここに書く形式で用意する。表の概念は :ref:`page-theory-aero-tables` と :ref:`page-theory-aero-coefficients` にある。
列の並び、単位、必須と任意の別、読み込み時の検査を示す。

.. _sec-aero-table-format:

係数表（``aero_table.csv`` と ``aero_table.json``）
------------------------------------------------------------------

係数表は、CSV 本体と、拡張子を ``.json`` に替えた同名の付随ファイル（メタデータ）の 2 つで 1 組である。
``aero`` が ``out/aero_table.csv`` と ``out/aero_table.json`` を書き、``aero.method = "table"`` では同じ形式の外部の表を読み込む（:ref:`sec-aero-table-format-external`）。
表の種類は付随ファイルの ``kind`` で決まり、``"axisymmetric"``\ （軸対称表。既定）と ``"general"``\ （一般表）がある。
表の概念と、軸対称表と一般表の関係は :ref:`sec-aero-table` と :ref:`page-theory-aero-coefficients` にある。

CSV の値は、``mach`` ``alpha_deg`` ``beta_deg`` の格子の値を除いて、指数表記で書かれる。桁数は、読み戻したときに元の倍精度の値とビットまで一致する最短の表記である（書いて読んでも値は変わらない）。
CSV は、1 行目が見出し、2 行目以降が格子点 1 つにつき 1 行で、空行は読み飛ばす。

.. _sec-file-axisym:

軸対称表の CSV
~~~~~~~~~~~~~~

見出しは次のとおりである。

.. code-block:: text

   mach,alpha_deg,cn,ca_on,ca_off,xcp,cna,damp_s0,damp_s1,damp_s2,cl_delta,clp,lag_l0,lag_l1,cl4,cnpa,cmpa

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
   * - ``lag_l0``
     - m
     - 遅れの和 :math:`L_0=(2/S)\int A\,\mathrm{d}x`\ （胴体とフィンの見かけの質量。:eq:`eq-lag-sums`）
   * - ``lag_l1``
     - m\ :sup:`2`
     - 遅れの和 :math:`L_1=(2/S)\int A\,x\,\mathrm{d}x`\ （ノーズ先端まわり。:math:`L_1/L_0` が見かけの質量の中心）
   * - ``cl4``
     - 無次元
     - 誘導ロールの係数 :math:`C_{l4}`\ （:math:`\Delta C_l=C_{l4}\alpha_t^2\sin4(\phi_c-\phi_f)`、:math:`\alpha_t` は rad。:eq:`eq-induced-roll`）
   * - ``cnpa``
     - 1/rad
     - Magnus 力の傾き :math:`C_{Np\alpha}^{*}`\ （:math:`C_{N,\mathrm{Magnus}}=C_{Np\alpha}^{*}\hat p\sin\alpha_t`。McCoy の :math:`pd/V` で無次元化した値の 2 倍。:eq:`eq-magnus`）
   * - ``cmpa``
     - 1/rad
     - 静的な圧力中心まわりの Magnus のモーメントの傾き :math:`C_{mp\alpha}^{*}`\ （:eq:`eq-magnus-moment`。0 なら Magnus 力が圧力中心に働くだけ）

既定の格子は、Mach 数が :math:`M=0,0.02,\dots,3.0`\ （151 点）、全迎角が :math:`\alpha_t=0^\circ,1^\circ,\dots,90^\circ`\ （91 点）である
（設定 ``aero.mach_min`` ``mach_max`` ``mach_step`` と ``alpha_max_deg`` ``alpha_step_deg``。:ref:`[aero] の設定項目 <sec-config-aero>`）。
飛翔計算は 0 から 90° の :math:`\alpha_t'` で表を引くので、既定の表では範囲を超えない。``alpha_max_deg`` を小さくした表で範囲を超えた迎角は外挿し、その結果は使えないもの（終了ステータス 3）とする（:ref:`sec-aero-table`、:ref:`sec-aero-interp`）。

``lag_l0`` と ``lag_l1`` は、部品積み上げ法では機体の形状から求め（:ref:`sec-damping`）、パネル法と CFD の表にも同じ形状の値を入れる。
``cl4``、``cnpa``、``cmpa`` は、部品積み上げ法、パネル法、CFD のどれも推算せず 0 を書く。値を使うには外部の表（``aero.method = "table"``）に書く。
5 列の値の飛翔計算での使い方は :ref:`sec-aero-unsteady` にある。

.. _sec-file-general:

一般表の CSV
~~~~~~~~~~~~

見出しは次のとおりである。

.. code-block:: text

   mach,alpha_deg,beta_deg,cx_on,cx_off,cy,cz,cl,cm,cn,cl_delta,clp,sq0,sq1,sq2,sr0,sr1,sr2,lag_l0,lag_l1,cnpa,cmpa

行は Mach 数が最も外側、迎角が次、横滑り角が最も内側の順に、全格子点を 1 行ずつ並べる。
``alpha_deg`` は迎角 :math:`\alpha`、``beta_deg`` は横滑り角 :math:`\beta` である（定義は :ref:`sec-aero-angles`）。
行は 3 つの格子値に続く 19 個の値を持つ（18 個と 15 個の形式は :ref:`sec-file-v1`）。

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
   * - ``lag_l0`` / ``lag_l1`` / ``cnpa`` / ``cmpa``
     - 軸対称表と同じ
     - 遅れの和と Magnus 力とモーメントの傾き（軸対称表の同名の列と同じ意味。``cmpa`` は等価な圧力中心まわり）

一般表では、単一の圧力中心 :math:`x_{cp}` や :math:`C_{N\alpha}` を飛翔計算に使わない。
6 成分の定義、符号、基準点は :ref:`page-theory-aero-coefficients` と :ref:`sec-aero-forces`、減衰の和の使い方は :eq:`eq-moment-damp` にある。
一般表は ``cl4`` の列を持たない。誘導ロールは :math:`C_l` の列に含める（軸対称表から展開した表では、展開のときに加える）。

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

付随ファイルは、表の基準量、格子、種類、入力の識別子を持つ JSON である。書き出すときは、次の項目をこの順に書く。

.. list-table::
   :header-rows: 1
   :widths: 24 14 62
   :class: longtable

   * - 項目
     - 読み込み
     - 内容
   * - ``format_version``
     - 任意
     - 形式のバージョン。書き出すときは常に 4。読み込み時はこの値を使わず、CSV の列数で形式を判定する（:ref:`sec-file-v1`）
   * - ``kind``
     - 任意
     - ``"axisymmetric"``\ （既定）または ``"general"``
   * - ``source_hash``
     - 必須
     - 入力を識別する文字列。``aero`` が書く表では、同じ機体と空力の設定で作った表かを区別するための値になる。外部の表では任意の文字列でよい
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
   * - ``fin_roll_deg``
     - 任意
     - 軸対称表の誘導ロール ``cl4`` の基準になる 1 枚のフィンのロール角 :math:`\phi_f` [deg]（:math:`+y_b` から :math:`+z_b` へ測る。:eq:`eq-induced-roll`）。既定は 0。有限でなければエラー
   * - ``fin_count``
     - 任意
     - フィンの枚数。``cl4`` は 4 枚（十字形）のときと、この項目がないときだけ使う。部品積み上げ法、パネル法、CFD の表と、形状から遅れの和を補った外部の表には、形状のフィンの枚数を書く
   * - ``extrapolation``
     - 必須
     - 迎角と横滑り角が表の範囲外のときの扱い。``"linear"``\ （線形外挿。符号と範囲を制限する）または ``"clamp"``\ （端の値で一定）。Mach 数は、この値にかかわらず端の値で一定（:ref:`sec-aero-interp`）

部品積み上げ法、パネル法、CFD の軸対称表の ``fin_cant_ref_deg`` は、設定したカントである。CFD の一般表では、``stl`` と ``step`` のとき設定したカント、``panel_mesh`` のとき 0 である（形状にカントがないため）。CFD の表の値は ``cfd_report.json`` の ``fin_cant_ref_deg`` にも出る。

読み込み時の検査
~~~~~~~~~~~~~~~~

表を読み込むときは、次を検査する。違反があるとエラーにして、メッセージにファイル名（行に関するものは行番号も）を示す。

* 付随ファイルと CSV が読めること。付随ファイルが無い、JSON として読めないときは、そのファイルの名前を示す。
* 基準面積 ``ref_area``、基準直径 ``ref_diameter``、全長 ``length`` が有限で正であること。``fin_cant_ref_deg`` と ``fin_roll_deg`` が有限であること。
* ``moment_ref`` が ``"nose_tip"`` であること。ほかの値は、シミュレータが重心まわりへの移し替えをノーズ先端まわりのモーメントから行うので受け付けない。
* ``machs`` と ``alphas_deg``\ （一般表では ``betas_deg`` も）が空でなく、有限で、厳密に増加すること。
* 迎角の格子が 0 を含むこと。軸対称表の ``alphas_deg`` は 0 から始まり（違えば ``alphas_deg of an axisymmetric table must start at 0 deg (the total angle of attack), found …: the small-angle coefficients would be extrapolated``）、
  一般表の ``alphas_deg`` と ``betas_deg`` は、最初の値と最後の値の間に 0 を含む（違えば ``alphas_deg of a general table must include 0 deg between its first and last value, found … to …: the small-angle coefficients would be extrapolated``。``betas_deg`` も同じ）。
  零揚力抗力、法線力傾斜、迎角 0 の圧力中心と静安定余裕（:ref:`sec-static-margin-general`）を、外挿ではなく表の値から求めるためである。
* CSV の行数が、``machs`` と ``alphas_deg``\ （一般表では ``betas_deg`` も）の積に一致すること。
* 各行の格子の座標が、付随ファイルの格子から決まる位置の値に一致すること。行は、Mach 数を最も外側、次に迎角、一般表では最も内側を横滑り角として並べる。
  つまり :math:`i` 番目の行の座標は、付随ファイルの軸から一意に決まる。並びが違う行は、行番号と期待される格子点を示してエラーにする（比較の許容は相対 :math:`10^{-6}`）。
* 格子の座標の列（``mach`` ``alpha_deg`` ``beta_deg``）を含むすべてのセルが有限であること。``NaN`` と ``inf`` は、座標の照合より先に調べ、``row 4 of …: column 1 (mach) is not finite: inf`` の形（行と列は例）で列の位置と見出しを示してエラーにする。
* 値が数として読めない行、列数が合わない行。

各行の値の数は見出しの列数に一致しなければならない。軸対称表は 17 列（格子の 2 列に値 15 個）、一般表は 22 列（格子の 3 列に値 19 個）の形式か、次節の形式である。

.. _sec-file-v1:

列の少ない形式の表
~~~~~~~~~~~~~~~~~~

次の列の組の表も読み込む。足りない列は 0 とする。

.. list-table::
   :header-rows: 1
   :widths: 22 18 60

   * - 表
     - 列数
     - 足りない列
   * - 軸対称表（バージョン 1）
     - 10（値 8 個）
     - ``cl_delta`` ``clp`` ``lag_l0`` ``lag_l1`` ``cl4`` ``cnpa`` ``cmpa``。付随ファイルに ``format_version`` がなくてもよい
   * - 軸対称表（バージョン 2）
     - 12（値 10 個）
     - ``lag_l0`` ``lag_l1`` ``cl4`` ``cnpa`` ``cmpa``
   * - 軸対称表（バージョン 3）
     - 16（値 14 個）
     - ``cmpa``
   * - 一般表（15 個の値）
     - 18
     - ``lag_l0`` ``lag_l1`` ``cnpa`` ``cmpa``
   * - 一般表（18 個の値）
     - 21
     - ``cmpa``

見出しは、バージョン 4 の見出しの先頭から、その列数までと一致しなければならない。

``aero.method = "table"`` で読み込んだ表のロール項がすべて 0 のとき（バージョン 1 の表を含む）は、``aero``、``sim``、``dispersion`` のどれでも、現在の STL のフィンから部品積み上げ法のロール項（:math:`C_{l\delta}` と :math:`C_{lp}`）を求めて表に補う。
補ったときは、端末に注意書きを表示する。形状を抽出できないときは、注意書きを表示してロール項を 0 のままにし、機体はロールしない。フィンがなければ補う値も 0 で、注意書きは出ない。
ロール項を含む表は、そのまま使う。補った値は、読み込んだ表のメモリ上だけに入り、元のファイルは書き換えない。

遅れと Magnus の列を持たない外部の表（上の表のバージョン 1、2 と 15 個の値の一般表）には、ロール項と同じく、現在の STL の形状から部品積み上げ法の遅れの和（胴体とフィンの見かけの質量。:ref:`sec-damping`）を求めて補い、
``the external aero table has no lag/Magnus columns: the apparent-mass (lag) sums were filled from the geometry (component build-up); Magnus force and induced roll are 0`` の警告を出す。
形状を抽出できないときは遅れの和を 0 のまま飛ばし、``the external aero table has no lag/Magnus columns and the geometry is unavailable: the flight has no apparent-mass (lag) terms, Magnus force or induced roll`` の警告を出す。
どちらの場合も Magnus 力とモーメント、誘導ロールは 0 で、回転による法線力（減衰の和から求める。:eq:`eq-cnq-force`）は働く。

**実測や CFD のピッチ減衰を使う表**：強制振動の風洞試験や CFD、実射の同定で得たピッチ減衰 :math:`C_{mq}+C_{m\dot\alpha}` は、局所的な迎角の項だけでなく、見かけの質量の遅れと対流の回転の項（:eq:`eq-lag-section`）をすでに含む。
この値を使う表では ``lag_l0`` と ``lag_l1`` を 0 にし、減衰を減衰の和に入れる。
0 は列を持つ形式（バージョン 3 以降）に明示して書く。明示した 0 の列は補わないが、列のない形式では形状から遅れの和が補われ、見かけの質量の項を二重に数える（そのときの警告はこの点を示す）。
基準の重心 :math:`x_{\mathrm{ref}}` で測った :math:`C_{mq}+C_{m\dot\alpha}`\ （:math:`qd/(2V)` あたり）から、重心まわりの和 :math:`K=-(C_{mq}+C_{m\dot\alpha})\,d^2/4` を作り、
法線力の傾きと圧力中心を表す :math:`S_0,S_1` はそのままに :math:`S_2=K+2x_{\mathrm{ref}}S_1-x_{\mathrm{ref}}^2S_0` とする（:eq:`eq-damping`）。こうすると、重心が動いたときの減衰の変化は局所的な迎角の和の形で近似される。
:math:`C_{mq}` と :math:`C_{m\dot\alpha}` が別々に分かっているときは、:math:`C_{m\dot\alpha}` を遅れの和で表してもよい。
:math:`\dot u_z=V\dot\alpha` で遅れのモーメントは :math:`C_{m\dot\alpha}=2(x_{\mathrm{ref}}L_0-L_1)/d^2` にあたるので、:math:`L_0=0`、:math:`L_1=-C_{m\dot\alpha}d^2/2` とする（重心によらない純粋なモーメントになり、対流の回転の力も 0 になる）。
遅れの和は対流の回転の項にも同じ値で効き、:math:`C_{mq}` に :math:`C_{m\dot\alpha}` と同じ値を足すので、減衰の和の側は :math:`K=-(C_{mq}-C_{m\dot\alpha})\,d^2/4` から :math:`S_2` を作る。

.. _sec-aero-table-format-external:

外部の表を作るときの要点
~~~~~~~~~~~~~~~~~~~~~~~~

外部の表（``aero.method = "table"``）は、上の形式のとおりに CSV と付随ファイルを用意する。使い方は :ref:`sec-usage-external-table` にある。確認する点は次のとおりである。

* 付随ファイルは CSV と同じ名前で拡張子が ``.json`` のファイルで、必須の項目（``source_hash`` ``ref_area`` ``ref_diameter`` ``length`` ``machs`` ``alphas_deg`` ``extrapolation``）をすべて書く。
* 軸対称表の ``alpha_deg`` は全迎角 :math:`\alpha_t`、一般表の ``alpha_deg`` は迎角 :math:`\alpha`、``beta_deg`` は横滑り角 :math:`\beta` である。軸対称表の ``cn`` は法線力係数、一般表の ``cn`` はヨーモーメント係数 :math:`C_n` で、意味が違う（:ref:`sec-aero-cn-cz`）。
* モーメントはノーズ先端まわり（``moment_ref = "nose_tip"``。ほかの値は読み込みでエラー）で、:math:`qSd` で無次元化する。
* 一般表は ``kind = "general"`` とし、``betas_deg`` を書く。格子は不等間隔でもよいが、各軸は厳密に増加させ、CSV の行は Mach 数、迎角、横滑り角の順（横滑り角が最も内側）に並べる。軸対称表も Mach 数、全迎角の順に並べる。
* 軸対称表の ``alphas_deg`` は 0 から始め、一般表の ``alphas_deg`` と ``betas_deg`` は 0 を含める。格子の座標を含むすべてのセルは有限にする（上の「読み込み時の検査」）。
* 格子の外の引き出しは、迎角・横滑り角の最初の格子点より小さい側も、最後の格子点より大きい側も、範囲外として数える（``aero_table_range`` の ``angle_beyond``。:ref:`sec-file-summary`）。格子点が 1 つだけの軸では、その値以外はすべて範囲外である。
* 軸対称な外部の表は、``aero.table_kind = "general"`` で一般表に展開できる（展開した表は書き出さない）。外部の表が最初から一般表なら、そのまま使う。
* ロール項がすべて 0 の表には、前節のとおり部品積み上げ法のロール項が補われる。遅れと Magnus の列のない形式の表には、形状の遅れの和が補われる。
* 軸対称表の誘導ロール ``cl4`` は、付随ファイルの ``fin_roll_deg`` のフィンの面を基準にした値で、フィンが 4 枚の機体（``fin_count``）にだけ使う。
  ``cl4`` を書くときは ``fin_count`` も書く。ないときは 4 枚とみなして警告を出す（``the aero table has an induced roll cl4 but no fin_count in its sidecar: the sin(4 phi) form of four fins is assumed``）。
  Magnus 力の ``cnpa`` は :math:`\hat p=pd/(2V)` についての傾き（McCoy の :math:`C_{Np\alpha}` の 2 倍）で、正の向きは :math:`\bm{u}\times\bm{e}_x` である（:eq:`eq-magnus`）。``cmpa`` は静的な圧力中心まわりの Magnus のモーメント（McCoy の :math:`C_{Mp\alpha}` の向き。:eq:`eq-magnus-moment`）である。
* CSV の見出しは、列の名前と順序が前節の見出しと一致していなければならない（大文字小文字と前後の空白は無視する）。名前や順序が違うと、読み込みで ``error[IY-A044]: aero table <path>: unexpected CSV header …`` のエラーにする。

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
値は迎角 4°、最も低い亜音速 Mach 数（既定は :math:`M=0`）の解である。``python/plot.py panel`` で作図する（:ref:`page-theory-panel`）。

