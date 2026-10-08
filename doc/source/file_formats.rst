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

CSV の値は、``mach`` ``alpha_deg`` ``beta_deg`` の格子の値を除いて、指数表記で書かれる。桁数は、読み戻したときに元の倍精度の値とビットまで一致する最短の表記である（書いて読んでも値は変わらない）。
CSV は、1 行目が見出し、2 行目以降が格子点 1 つにつき 1 行で、空行は読み飛ばす。
``aero`` は、CSV と付随ファイルを一時ファイルに書いてから名前を付け替えて置くので、書き込みの途中で止まっても、壊れた表がキャッシュに残らない。

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
行は 3 つの格子値に続く 19 個の値を持つ（18 個と 15 個の古い形式は :ref:`sec-file-v1`）。

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
6 成分の定義、符号、基準点は :doc:`aero_coefficients` と :ref:`sec-aero-forces`、減衰の和の使い方は :eq:`eq-moment-damp` にある。
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
     - 形式のバージョン。書き出すときは常に 4。読み込み時はこの値を使わず、CSV の列数で形式を判定する（:ref:`sec-file-v1`）
   * - ``kind``
     - 任意
     - ``"axisymmetric"``\ （既定）または ``"general"``
   * - ``source_hash``
     - 必須
     - 入力のハッシュ値。``aero`` は STL の内容、抽出した形状（縮尺、機首方向、フィンの上書き、カントを適用した後のもの）、設定値、プログラムのバージョンから計算した FNV-1a の値（16 桁の 16 進数）を書き、保存済みの表を使うかの判定に使う。外部の表では任意の文字列でよい
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

各行の値の数は見出しの列数に一致しなければならない。軸対称表は 17 列（格子の 2 列に値 15 個）、一般表は 22 列（格子の 3 列に値 19 個）の形式か、次節の古い形式である。

.. _sec-file-v1:

古い列の組の表
~~~~~~~~~~~~~~

次の列の組の表も読み込む。足りない列は 0 とする。どの表も、保存するときは現在の形式（バージョン 4）で書く。

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

見出しは、現在の見出しの先頭から、その列数までと一致しなければならない。

``aero.method = "table"`` で読み込んだ表のロール項がすべて 0 のとき（バージョン 1 の表を含む）は、``aero``、``sim``、``dispersion`` のどれでも、現在の STL のフィンから部品積み上げ法のロール項（:math:`C_{l\delta}` と :math:`C_{lp}`）を求めて表に補う。
補ったときは、端末に注意書きを表示する。形状を抽出できないときは、注意書きを表示してロール項を 0 のままにし、機体はロールしない。フィンがなければ補う値も 0 で、注意書きは出ない。
ロール項を含む表は、そのまま使う。補った値は、読み込んだ表のメモリ上だけに入り、元のファイルは書き換えない。

遅れと Magnus の列を持たない外部の表（上の表のバージョン 1、2 と 15 個の値の一般表）には、ロール項と同じく、現在の STL の形状から部品積み上げ法の遅れの和（胴体とフィンの見かけの質量。:ref:`sec-damping`）を求めて補い、
``the external aero table has no lag/Magnus columns: the apparent-mass (lag) sums were filled from the geometry (component build-up); Magnus force and induced roll are 0`` の警告を出す。
形状を抽出できないときは遅れの和を 0 のまま飛ばし、``the external aero table has no lag/Magnus columns and the geometry is unavailable: the flight has no apparent-mass (lag) terms, Magnus force or induced roll`` の警告を出す。
どちらの場合も Magnus 力とモーメント、誘導ロールは 0 で、回転による法線力（減衰の和から求める。:eq:`eq-cnq-force`）は働く。

**実測や CFD のピッチ減衰を使う表**：強制振動の風洞試験や CFD、実射の同定で得たピッチ減衰 :math:`C_{mq}+C_{m\dot\alpha}` は、局所的な迎角の項だけでなく、見かけの質量の遅れと対流の回転の項（:eq:`eq-lag-section`）をすでに含む。
この値を使う表では ``lag_l0`` と ``lag_l1`` を 0 にし、減衰を減衰の和に入れる。
0 は列を持つ形式（バージョン 3 以降）に明示して書く。明示した 0 の列は補わないが、列のない古い形式では形状から遅れの和が補われ、見かけの質量の項を二重に数える（そのときの警告はこの点を示す）。
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
* CSV の見出しは、列の名前と順序が前節の見出しと一致していなければならない（大文字小文字と前後の空白は無視する）。名前や順序が違うと、読み込みで ``unexpected CSV header`` を含むエラーにする。

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
     - 履歴の最初の行と、判定した行の密度残差（対数）。判定した行は、ランナーが収束を見て SU2 を止めたときはその時点の行、止めなかったときは最後の行で、SU2 が止まるまでに書き足した行は含めない（``iterations`` は書き足した行も数える）。RANS では 2 次精度の段の履歴の値である
   * - ``cn_raw`` / ``ca_cfd``
     - SU2 の生の法線力係数 :math:`C_N` と軸力係数 :math:`C_A`\ （RANS の :math:`C_A` は壁面の ``CFx(wall)``、``rans_base = "cfd"`` では底面の ``CFx(base)`` を足した値。:ref:`sec-cfd-post`）
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
     - 警告の一覧
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

``done.json``
~~~~~~~~~~~~~

各ケースのディレクトリ（``<workdir>/m0.800_a04.00/`` など）に、ケースが終わると書かれる。入力のハッシュが同じなら、次の実行でこのケースを解かずに使う（:ref:`sec-cfd-int-convergence`）。

.. list-table::
   :header-rows: 1
   :widths: 30 70
   :class: longtable

   * - 項目
     - 内容
   * - ``input_hash``
     - SU2 の設定（暖機起動の行を除く）とメッシュのハッシュ
   * - ``mach`` / ``alpha_deg`` / ``beta_deg`` / ``roll_hat``
     - ケースの条件
   * - ``coeffs`` / ``coeffs6``
     - 機体軸の係数（``cn``、``ca``、``mom``）と 6 成分（``cx`` ``cy`` ``cz`` ``cl`` ``cm`` ``cn``）。半領域のケースでは、``cy``、``cl``、``cn`` は 0
   * - ``iterations`` / ``converged`` / ``residual_first`` / ``residual_last`` / ``wall_seconds``
     - ``cfd_cases.csv`` の同じ名前の列と同じ。``converged`` は、Euler では残差が目標に達し、係数の傾きも落ち着いたとき、RANS では収束の判定（:ref:`sec-cfd-int-rans-accept`）を満たしたとき真
   * - ``reference_peak``
     - RANS のとき、受け入れの基準にした残差の最大値（``rho`` が密度、``nu`` が SA の残差。対数）。暖機起動の次のケースに受け継ぐ
   * - ``limit_cycle``
     - RANS で、振動または停滞として窓の平均を受け入れたとき ``true``\ （``false`` のときは書かない）
   * - ``stalled`` / ``residual_drop``
     - RANS で、残差が停滞したまま受け入れたとき ``stalled`` が ``true``\ （``false`` のときは書かない。``limit_cycle`` も ``true``）。``residual_drop`` は判定の根拠にした密度の残差の下がり [桁]（``cfd_cases.csv`` の同じ名前の列と同じ）
   * - ``yplus_p95``
     - RANS のとき、壁面の :math:`y^+` の 95 パーセンタイル（底面を除く）
   * - ``axial``
     - RANS のとき、``wall_cfx`` と ``base_cfx``\ （壁面と底面の ``CFx``）、``base_in_cfx``\ （``coeffs`` の軸力が底面の力を含むか）、``markers_missing``\ （履歴にマーカごとの列がなかったか）
   * - ``warnings``
     - 受け入れたケースについての警告（壁面の :math:`y^+`、振動や停滞、暖機起動や発散からの解き直しなど）。ないときは書かない

.. _sec-file-mesh:

``<workdir>/mesh/`` のファイル
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

CFD のメッシュのキャッシュで、``mesh.su2``、メッシュを作ったときの設定 ``params.json``、ログ ``mesh.log``、メッシュの統計 ``mesh_stats.json`` などが入る。

``mesh_stats.json``
   ``nodes``、``cells``\ （節点数とセル数）、``by_type``\ （セルの種類ごとの数）、``quality_sicn_min``、``quality_sicn_mean``、``cells_sicn_below_0.05``\ （SICN の最小値、平均、0.05 未満のセル数。RANS のメッシュでは四面体だけについて求める）、
   ``wall_triangles``、``seconds_setup``、``seconds_mesh`` を持つ。RANS のメッシュでは、プリズム層について次の項目も持つ（:ref:`sec-cfd-prism`）。

   * ``bl_layers`` / ``bl_first_height`` / ``bl_thickness``：層の数、最初の層の高さ [m]、最も厚い柱の厚さ [m]
   * ``bl_invalid``：書いたメッシュの不正なプリズムの数（不正な層はメッシュのエラーになるので、常に 0）
   * ``bl_attempt``：何回目の作成で検査に通ったか（1 または 2）
   * ``bl_top_ratio_p99`` / ``bl_top_ratio_max``：層の上面の四面体の高さと最後の層の高さの比（大きいほうを小さいほうで割った値）の 99 パーセンタイルと最大値

``surface = "stl"`` では、次のファイルも入る。

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

``surface = "step"`` では、次のファイルも入る。

``step_prepared.brep``
   単位をそろえ、CFD の座標に移し、Euler の ``"fairing"`` では円すいを融合した CAD の立体（OpenCASCADE の BREP、m 単位）。体積メッシュはこれから作る。

``step_surface.stl``
   準備した立体の三角形分割（面ごとにまとめた二進形式の STL、CFD の座標、m 単位）。位置合わせに使う。

``step_report.json``
   読み込みの報告。``solids_in_file`` と ``faces_in_file``\ （ファイルのソリッドと面の数）、``notes``\ （縫い合わせ、修復、融合の注記）、``solids``、``volume``、``area``、``bbox``、
   ``faces``\ （面ごとの ``tag``、``type``、``area``、``centroid``、``normal``、``planar``、``width``\ （:math:`2A/P` [m]）、``bbox``、``base``）、``face_triangles``\ （面ごとの三角形の数）、
   ``smallest_face_width`` と ``smallest_face``\ （最も細い面の幅とその面）、``smallest_edge`` と ``smallest_edge_tag``\ （最も短い辺の長さとその辺）、``symmetry_y`` と ``symmetry_z``\ （平面ごとの鏡映の検査）、``fairing``\ （円すいの位置と大きさ、または付けられなかった理由 ``error``）、``seconds`` と ``seconds_symmetry``\ （準備の段全体と対称性の検査の時間 [s]）を持つ。

``step_prepare.json`` / ``prepare_params.json`` / ``prepare.log``
   準備の段のハッシュ、その設定、ログ。ハッシュが同じなら準備の段をやり直さない。

``step_quarter_failed.json``
   1/4 領域を切り出せなかったときに、準備した形状のハッシュを書く。同じ形状では最初から半領域をメッシュにする（:ref:`sec-cfd-step-internals`）。

``ignisyeet cfd-check 設定ファイル --mesh`` は、``surface = "step"`` のとき、メッシュの統計のあとに ``CAD surface (surface = "step", base treatment: …):`` で始まる報告を表示する。
行は ``file``、``shape``、``details``、``domain``\ （対称性の判定の 2 行を含む）、``base``、``registration`` 以下の位置合わせの 4 行で、``note:`` と ``warning:`` の行が続くことがある。
各行の意味と例は :ref:`読み込みと位置合わせの報告 <sec-cfd-step-report>` にある。値は ``cfd_report.json`` の ``step`` と同じである。

形状と断面のファイル
--------------------

.. _sec-file-geometry:

``geometry.json``
~~~~~~~~~~~~~~~~~

``geom`` と、係数表を作る ``aero`` が書く、抽出した形状である。
全長 ``length``、基準半径 ``ref_radius`` と基準面積 ``ref_area``、底面半径 ``base_radius``、ノーズ ``nose``、ボートテール ``boattail``、胴体のぬれ面積 ``wetted_area_body`` と体積 ``volume``、
フィンの平面形の面積 ``planform_area`` と重心位置 ``planform_centroid``、フィン ``fins``、STL の座標との対応 ``frame``\ （``axis_point``、``aft_direction``、``nose_offset``）、断面ごとの ``profile`` を持つ。
機体軸 :math:`x_b,y_b,z_b` を STL の座標で表した単位ベクトル ``body_x``、``body_y``、``body_z`` も書く（:ref:`sec-body-basis`）。
抽出の妥当性についての所見は、文字列の配列 ``warnings`` に書く（疑わしい点がなければ空。:ref:`sec-geometry-warnings`）。

``profile.csv``
~~~~~~~~~~~~~~~

見出しは ``x,r,fin_span`` で、断面の位置 ``x``、胴体半径 ``r``、フィン高さ ``fin_span`` [m] を断面ごとに書く。

飛翔と落下分散の出力
--------------------

飛翔と落下分散の出力は :doc:`usage` の「出力ファイル」で概要を述べる。ここでは列と項目を示す。

.. _sec-file-summary:

``summary.json``
~~~~~~~~~~~~~~~~

``sim`` が書く、1 回の飛翔の要約である。
降下モード ``descent``、基準風 ``wind_speed`` と ``wind_direction_deg``\ （``profile`` では表の平均風速 :math:`W` と平均風の風向 :math:`\Psi`。:ref:`sec-wind-profile`）、ランチャ離脱の時刻 ``rail_exit_time`` と速度 ``rail_exit_speed``、燃焼終了時刻 ``burnout_time``\ （推力曲線の終わりの時刻。飛翔がそれより前に終わったときは ``null``）、
最大速度 ``max_speed``、最大 Mach 数 ``max_mach``、最大動圧 ``max_dyn_pressure``\ （積分のステップの始端ごとの状態と、着地の状態のうちの最大）、頂点の時刻 ``apogee_time`` と高度 ``apogee``\ （射点からの高さ）と位置 ``apogee_east`` ``apogee_north``、
最小静安定余裕 ``min_stability_cal`` とランチャ離脱時の値 ``stability_at_rail_exit``\ （どちらも迎角 0 の極限の小迎角の余裕で、その状態が飛んでいる迎角は入らない。:ref:`sec-static-margin-general`。``min_stability_cal`` は、ランチャ離脱の時刻（自由飛行はここから始まる）から頂点までのうち対気速度が 10 m/s を超える間に、積分のステップの始端ごとに測った最小値である。
離脱時の対気速度が 10 m/s を超えるときは、ランチャ離脱の時刻の値を含む。頂点付近の遅い部分と降下は含まない。
該当する時刻がなければ（頂点までに対気速度が 10 m/s を超えない飛翔など）``null`` である。``stability_at_rail_exit`` は対気速度によらず求める。軸対称表では小さな迎角の圧力中心から求めた :math:`(x_{cp}(\delta)-x_{cg})/d`\ （:eq:`eq-margin-axisym`）、一般表では各ステップのピッチとヨーの余裕の小さいほう）、
ランチャ離脱の値と着地の値は、ステップ内の補間（位置は三次エルミート補間、速度はその時間微分、残りは両端の線形補間）で求めた状態の値である（:ref:`sec-rail`、:ref:`sec-landing`）、
着地の時刻 ``landing_time``、位置 ``landing_east`` ``landing_north`` ``landing_lat`` ``landing_lon``、射点からの距離 ``landing_distance``、着地速度 ``landing_speed`` を持つ。
``landing_speed`` は着地の瞬間の **地面に対する速さ** で、水平方向の流され（風による移動）の速さを含む。降下率（鉛直の速さ）ではないので、パラシュート降下で風が強いと、降下率より大きい。
これに加えて、次の項目を書く。

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - 項目
     - 内容
   * - ``landed``
     - ``sim.max_time`` までに着地したか（真偽値）。``false`` のとき、着地に関する 7 項目（``landing_time`` から ``landing_speed`` まで）は ``null`` である
   * - ``main_deploy_time`` / ``main_deploy_altitude``
     - 二段開傘（``recovery.main_cd_s`` と ``recovery.main_altitude``）のメインが開傘した時刻 [s] と射点からの高さ [m]（:ref:`sec-dual-deploy`）。
       メインがない設定、弾道落下、メインの開傘より前に飛翔が終わった場合は ``null`` である
   * - ``static_margin_min_cal``
     - ``min_stability_cal`` と ``stability_at_rail_exit`` のうち、求まったものの小さいほう [cal]（結果パネルの ``min margin`` と同じ）。
       ``min_stability_cal`` が ``null``\ （対気速度が 10 m/s を超えなかった）なら ``stability_at_rail_exit`` の値で、どちらも求まらなければ ``null``
   * - ``static_margin_min_t`` / ``static_margin_min_mach``
     - 最小の静安定余裕をとった状態の時刻 [s] と Mach 数（:ref:`sec-static-margin-general`）。余裕を一度も求めなかった飛翔では ``null``
   * - ``stable``
     - 静的に安定なら ``true``、不安定（求まった余裕のどれかが負）なら ``false``、判定できない（求まらない余裕がある、またはどの余裕も求めなかった）なら ``null``。
       ``false`` と ``null`` のとき、``sim`` は着地したかどうかにかかわらず終了ステータス 3 で終わる（:ref:`sec-exit-status`）
   * - ``stability_unknown``
     - ``stable`` が ``null`` のときだけ書く。判定できない理由（``at t = 0.50 s: the yaw normal-force slope dCy/dbeta is zero at Mach 0.10`` の形で、最初に余裕が求まらなかった時刻と理由。:ref:`sec-static-margin-general`）
   * - ``static_margin_pitch_min_cal`` / ``static_margin_yaw_min_cal``
     - ピッチ面とヨー面の静安定余裕の、自由飛行中とランチャ離脱時の最小値 [cal]（:ref:`sec-static-margin-general`）。軸対称表では両方とも ``static_margin_min_cal`` と同じ値である。
       その面の余裕がどこかで求まらなければ ``null``
   * - ``drogue_deploy_time``
     - （ドローグ、または単一の）パラシュートが開いた時刻 [s]。開傘は、頂点の時刻に ``recovery.delay`` を足した時刻に、刻み幅によらず正確に起こる（:ref:`sec-integrators`）。その時刻にモータがまだ燃えていれば、推力の終わりの時刻に遅らせる（``deploy_delayed``）。弾道落下と、開傘する前に着地した飛翔では、この項目を書かない
   * - ``deploy_delayed``
     - 開傘を推力の終わりまで遅らせたときだけ書く。本来の開傘時刻（頂点 + ``recovery.delay``）``nominal_time`` [s] と、その時刻の推力 ``thrust`` [N] を持つ（:ref:`sec-parachute`）
   * - ``broadside_time`` / ``fell_broadside``
     - 頂点のあとの弾道落下で、全迎角が 60° から 120° の間で飛んだ時間 [s]（パラシュート降下では 0）と、それが 10 s を超えたか（真偽値）。``true`` のときは警告を出す（:ref:`sec-lim-aero`）
   * - ``roll_resonance``
     - ロール角速度がピッチの固有振動数を横切ったときだけ書く（トリムの強制がある飛翔で、頂点より前の自由飛行。:ref:`sec-roll-resonance`）。
       最初の交差の時刻 ``t`` [s]、Mach 数 ``mach``、そのときのロール角速度（＝固有振動数）``frequency`` [rad/s]、上昇中の交差の回数 ``crossings`` を持つ。警告も出す（:ref:`sec-config-checks`）
   * - ``drogue_deploy_speed`` / ``drogue_deploy_q``
     - パラシュート（二段開傘ではドローグ）が開いたときの対気速度 [m/s]\ （風に対する速さ）と動圧 [Pa]。``drogue_deploy_time`` と同じく、開傘しなかった飛翔では書かない。
       対気速度が 50 m/s を超えるか動圧が 2 kPa を超えると警告する（:ref:`sec-config-checks`）
   * - ``main_deploy_speed`` / ``main_deploy_q``
     - 二段開傘のメインが開いたときの対気速度 [m/s] と動圧 [Pa]。メインが開かなかった飛翔では書かない
   * - ``earth_model``
     - 地球のモデル ``earth.model``\ （``"flat"`` または ``"ecef"``）。``python/plot.py`` は、これで高度の求め方を選ぶ（:ref:`sec-run-first`）
   * - ``wind_model``
     - 風のモデル ``wind.model``\ （``"constant"``、``"power"``、``"log"``、``"profile"``）
   * - ``wind_scale``
     - ``wind.model = "profile"`` のときだけ書く。その飛翔で表の風速に掛けた倍率 :math:`k`\ （CSV の表そのままなら 1。風の格子とモンテカルロのケースは、そのケースの平均風速に合わせた倍率。:ref:`sec-wind-scaling`）
   * - ``wind_at_ref_height``
     - ``wind.model = "profile"`` のときだけ書く。公称の表の風を ``wind.ref_height`` で求めた値で、高さ ``height`` [m]、風速 ``speed`` [m/s]、吹いてくる方位 ``direction_deg`` [deg] を持つ（無風なら方位は 0）。
       団体の風速の上限を測る高さが ``wind.ref_height`` なら、保安距離の手順 1 の格子の上限は :math:`v_{\max}=W\,V_{\mathrm{lim}}/` ``speed``\ （:math:`W` は ``wind_speed``）になる（:ref:`sec-dispersion-safety`）。
       ``sim`` と ``dispersion`` は、端末にも ``profile wind at wind.ref_height = 2 m: 3.04 m/s from 10 deg (mean wind 11.89 m/s)`` の形の行を出す（例の表の値）
   * - ``max_time``
     - 打ち切り時刻 ``sim.max_time`` [s]
   * - ``config_hash``
     - この結果を計算した設定のハッシュ（16 桁の 16 進数）。出力先と計算資源（``[output]`` と ``[resources]``）、``[dispersion]`` を除く設定と、STL、モータ、風のプロファイル、外部の係数表（CSV と、基準面積・基準直径・全長などを持つ付属の JSON）の内容から計算する。KML は、現在の設定のハッシュと一致するものだけをまとめる（:ref:`sec-kml`）
   * - ``warnings``
     - 警告の文字列の配列（空のこともある。:ref:`sec-config-checks`）。設定の読み込み時の警告（``profile`` で使わない ``wind.speed`` と ``wind.direction_deg``、2 回以上指定した ``--set`` のキー）が先頭で、計算中の警告が続く。``dispersion_meta.json`` と ``dispersion_summary.json`` の ``warnings`` も同じ順である
   * - ``aero_table_range``
     - 係数表の範囲外の引き出しの集計。全引き出しの回数 ``lookups``、Mach 数が表の最小値未満・最大値超の回数 ``mach_below`` ``mach_above``、迎角または横滑り角が格子の外の回数 ``angle_beyond``、要求された最大の Mach 数 ``max_mach`` と最大の迎角 ``max_abs_alpha_deg`` [deg]（軸対称表では引いた全迎角、一般表では :math:`\alpha` と :math:`\beta` から求めた全迎角 :math:`\arccos(\cos\alpha\cos\beta)`）、範囲外が 1 回でもあったか ``out_of_range``\ （真偽値）、迎角の範囲外がなかったか ``angles_in_range`` と Mach 数の範囲外がなかったか ``machs_in_range``\ （真偽値。どちらかが ``false`` のときは結果を使えず、終了ステータス 3 で終わる。:ref:`sec-exit-status`）

``sim`` は、実行の最初に、出力先の古い ``trajectory.csv`` と ``summary.json`` を消す。実行が失敗したとき（積分の発散、``launch.elevation_deg`` が範囲外、ランチャを離れないなど）は、
``summary.json`` を失敗の印 ``{"failed": true, "error": "…"}`` だけで書き、``error`` にエラーの文を入れる。``trajectory.csv`` は書かない。``ignisyeet.kml`` があれば、飛翔の層を除いて書き直す（:ref:`sec-kml`）。
古い結果が残って新しい結果と取り違えられないようにするためで、``python/plot.py`` は、この印があるとき飛翔経路を描かずにエラーで終わる（:ref:`sec-run-first`）。

.. _sec-file-trajectory:

``trajectory.csv``
~~~~~~~~~~~~~~~~~~

見出しは次のとおりである。

.. code-block:: text

   t,phase,east,north,up,lat,lon,alt,vel_e,vel_n,vel_u,airspeed,mach,alpha_deg,thrust,mass,x_cg,x_cp,stability_cal,dyn_pressure,cn,ca,pitch_deg,heading_deg,wx,wy,wz,beta_deg,roll_rate,cl,cy

時刻歴で、位置（``east`` ``north`` ``up``、``lat`` ``lon`` ``alt``）、速度、``airspeed``、``mach``、``thrust``、``mass``、``x_cg``、``x_cp``、``stability_cal``、``dyn_pressure``、姿勢と角速度を持つ。
``x_cp`` と ``stability_cal`` は、その状態が飛んでいる迎角での圧力中心と :math:`(x_{cp}-x_{cg})/d` [cal] の表示である。迎角が大きいと圧力中心が前に動くので、ランチャ離脱の直後や頂点付近では小さくなる。
静的な安定の判定と ``summary.json`` の余裕は、迎角 0 の極限の小迎角の余裕で、この列とは違う（:ref:`sec-static-margin-general`）。
``phase`` は ``rail``、``free``、``parachute`` のいずれかである。
``alpha_deg`` は全迎角 :math:`\alpha_t`、``x_cp`` は一般表では等価な圧力中心（:eq:`eq-xcp-eff`）で、一般表の ``stability_cal`` はこの値から求める。
末尾の 4 列は、``beta_deg``\ （横滑り角 :math:`\beta` [deg]）、``roll_rate``\ （ロール角速度 :math:`\omega_x` [rad/s]）、
``cl``\ （ロールの係数 :math:`C_l^{\mathrm{tot}}`、:eq:`eq-cl-total`）、``cy``\ （一般表の :math:`C_Y`。軸対称表では 0）である。

``dispersion.csv``
~~~~~~~~~~~~~~~~~~

``mode = "wind_grid"`` が書く。見出しは次のとおりである。

.. code-block:: text

   descent,wind_speed,wind_direction_deg,landing_east,landing_north,landing_lat,landing_lon,landing_distance,landing_time,landing_speed,apogee,apogee_time,max_mach,rail_exit_speed,min_stability_cal,landed,error,stable,broadside_time

風の各ケースの着地点（ENU と緯度経度）、頂点、飛行時間などである。
``wind_speed`` と ``wind_direction_deg`` は格子の値で、``constant`` ``power`` ``log`` では基準高度の風速と風向、``profile`` では表の平均風速 :math:`W` と平均風の風向 :math:`\Psi` に対する値である（:ref:`sec-wind-scaling`）。
``landing_speed`` は着地の瞬間の地面に対する速さである（:ref:`sec-file-summary`）。
末尾の ``landed`` は、そのケースが ``sim.max_time`` までに着地したかを ``true`` または ``false`` で示す。
着地しなかったケースでは、着地に関する列（``landing_east`` から ``landing_speed`` まで）は ``NaN`` になる。
列 ``error`` は、積分がエラーで止まったケースのエラーの文である（改行は空白に替え、コンマや引用符を含むときは引用符で囲む）。エラーのないケースでは空である。
エラーで止まったケースは、``landed = false`` で、``wind_speed`` と ``wind_direction_deg`` 以外の数値の列が ``NaN`` になる（:ref:`sec-wind-grid-failed`）。
列 ``stable`` は、そのケースが静的に安定か（``summary.json`` の ``stable`` と同じ判定で、安定なら ``true``、不安定なら ``false``）である。判定できないケースと、エラーで止まったケースでは空である（:ref:`sec-dispersion-unstable`）。
最後の列 ``broadside_time`` は、``summary.json`` と同じ、頂点のあとの弾道落下で全迎角が 60° から 120° の間で飛んだ時間 [s]（小数 2 桁）である。パラシュート降下の行では 0、エラーで止まったケースでは空である。10 を超える弾道落下の行が、横向きに落ちたケースである（:ref:`sec-dispersion-broadside`）。
一般表では、``min_stability_cal`` は各ステップのピッチとヨーの余裕の小さいほうの最小値（:ref:`sec-static-margin-general`）である。

.. _sec-file-dispersion:

``dispersion_meta.json``
~~~~~~~~~~~~~~~~~~~~~~~~

``mode = "wind_grid"`` が ``dispersion.csv`` と並べて書く。項目は、設定のハッシュ ``config_hash``\ （風の格子について、``mode`` と ``[dispersion.monte_carlo]`` を除いた設定から計算する）、
飛翔の設定のハッシュ ``flight_config_hash``\ （``summary.json`` の ``config_hash`` と同じ計算で、``sim`` の結果と同じ設定かを確かめるのに使う）、
ケース数 ``cases``、着地しなかったケースの数 ``failed``、そのうち積分がエラーで止まったケースの数 ``errored``、警告の文字列の配列 ``warnings``、係数表の範囲外の引き出しの集計 ``aero_table_range``\ （項目は :ref:`sec-file-summary` と同じ。エラーで止まったケースは数えない）、
公称の飛翔の最小の静安定余裕 ``static_margin_min_cal`` [cal] と、それが 0 以上かを示す ``stable``\ （真偽値）、
降下モード（``ballistic`` と ``parachute``）ごとの静的に不安定なケースの数 ``unstable`` と、そのモードのケースの最小の静安定余裕 ``unstable_min_static_margin_cal`` [cal]\ （余裕が求まるケースがなければ ``null``）、
降下モードごとの静的な安定を判定できないケースの数 ``stability_unknown_runs`` と、公称の飛翔が判定できないときの理由 ``stability_unknown``\ （判定できたときは ``null``）、
横向きに落ちた（``broadside_time`` が 10 s を超えた）弾道落下のケースの数 ``broadside_runs``\ （``{"ballistic": 1}`` の形。弾道落下だけを数える）、
公称の風のモデル ``wind_model``、公称の基準風速 ``wind_speed`` と基準風向 ``wind_direction_deg``\ （``profile`` では表の平均風速 :math:`W` と平均風の風向 :math:`\Psi`。``profile_mean_speed`` と ``profile_mean_direction_deg`` を指定したときはその値）、
``profile`` のときだけ公称の表の倍率 ``wind_scale``\ （CSV の風速に掛ける倍率。格子の風速はここからさらに拡大する。:ref:`sec-wind-scaling`）と ``wind.ref_height`` での公称の表の風 ``wind_at_ref_height``\ （``summary.json`` と同じ）である。
公称の飛翔は、設定した風での弾道落下である。公称の飛翔が失敗して余裕が求まらないときは、``static_margin_min_cal`` と ``stable`` は ``null`` である。公称の飛翔の静的な安定を判定できないときも ``stable`` は ``null`` である。
``stable`` が ``false`` か ``null`` のとき（公称の飛翔が失敗した場合を除く）、``unstable`` または ``stability_unknown_runs`` が 1 以上のモードがあるとき、迎角または Mach 数の範囲外の引き出しがあったとき（``aero_table_range`` の ``angles_in_range`` または ``machs_in_range`` が ``false``）、``dispersion`` は終了ステータス 3 で終わる（:ref:`sec-exit-status`、:ref:`sec-dispersion-unstable`）。
KML は、このファイルのハッシュが現在の設定と一致するときだけ ``dispersion.csv`` を読み込む。

``dispersion_mc.csv``
~~~~~~~~~~~~~~~~~~~~~

``mode = "monte_carlo"`` が書く、サンプルごと・降下モードごとの 1 行である。見出しは次のとおりである。

.. code-block:: text

   sample,descent,status,thrust_scale,burn_time_scale,dry_mass,cg,cn_scale,ca_scale,elevation_deg,azimuth_deg,roll_deg,wind_speed_delta,wind_direction_deg_delta,parachute_cd_s_scale,wind_speed,wind_direction_deg,landing_east,landing_north,landing_lat,landing_lon,landing_distance,landing_time,landing_speed,apogee,apogee_time,max_mach,rail_exit_speed,min_stability_cal,thrust_misalignment_y_deg,thrust_misalignment_z_deg,thrust_offset_y,thrust_offset_z,fin_cant_deg,cg_lateral_y,cg_lateral_z,cl_scale,clp_scale,lag_scale,magnus_scale,cmq_scale,stable,broadside_time

``status`` は ``ok`` または ``failed`` で、失敗した行は結果の列が空欄になる。``sim.max_time`` までに着地しなかったサンプルも ``failed`` である。
``thrust_scale`` から ``parachute_cd_s_scale`` までは引いたばらつきで、相対のものは割合、``wind_speed_delta`` と ``wind_direction_deg_delta`` は風速・風向のずれである。
``elevation_deg`` と ``azimuth_deg`` は、引いた値そのものではなく、実際に飛ばしたレールの射角と射方位の公称値からの変化 [deg]、``roll_deg`` は機体のロール角の公称値からの変化 [deg] である（射方位とロール角は :math:`(-180^\circ,180^\circ]` に折り返す。:ref:`sec-mc-rail`）。
``wind_speed`` と ``wind_direction_deg`` はそのサンプルの基準風速・風向（``profile`` では表の平均風速 :math:`W` と平均風の風向 :math:`\Psi` を拡大・回転した値）、続く列は着地点（ENU と緯度経度）、着地距離、着地時刻、着地速度、頂点、頂点時刻、最大 Mach 数、ランチャ離脱速度、最小静安定余裕である。
続く 9 列（``thrust_misalignment_y_deg`` から ``clp_scale``）は、ロールと非対称について引いた値、その次の 3 列（``lag_scale``、``magnus_scale``、``cmq_scale``）は空力の遅れ、Magnus 力、減衰の和について引いた割合である（:ref:`sec-mc`）。
列 ``stable`` は、そのサンプルが静的に安定か（``summary.json`` の ``stable`` と同じ判定で、安定なら ``true``、不安定なら ``false``）で、判定できないサンプルと失敗したサンプルでは空である（:ref:`sec-dispersion-unstable`）。
最後の列 ``broadside_time`` は ``dispersion.csv`` と同じで、失敗したサンプルでは空である。

``dispersion_summary.json``
~~~~~~~~~~~~~~~~~~~~~~~~~~~

``mode = "monte_carlo"`` が書く。全体の項目はサンプル数 ``samples``、乱数の種 ``seed``、標準正規乱数を切る範囲 ``truncate_sigma``\ （``dispersion.monte_carlo.truncate_sigma`` の値。切らないときは ``null``。:ref:`sec-mc`）、降下モードごとの配列 ``descents``、設定のハッシュ ``config_hash``\ （モンテカルロについて、``mode`` と風の格子の設定を除いた設定から計算する）、飛翔の設定のハッシュ ``flight_config_hash``\ （``summary.json`` の ``config_hash`` と同じ計算）、警告の配列 ``warnings``、係数表の範囲外の引き出しの集計 ``aero_table_range``\ （項目は :ref:`sec-file-summary` と同じ。失敗したサンプルは数えない）、
公称の飛翔の最小の静安定余裕 ``static_margin_min_cal`` [cal] と、それが 0 以上かを示す ``stable``\ （風の格子の ``dispersion_meta.json`` と同じ。公称の飛翔が失敗したときは ``null``）、
降下モードごとの静的に不安定なサンプルの数 ``unstable`` と最小の静安定余裕 ``unstable_min_static_margin_cal``\ （風の格子の ``dispersion_meta.json`` と同じ。失敗したサンプルは余裕を持たないので、数に入らない）、
``stability_unknown_runs`` と ``stability_unknown``、横向きに落ちた弾道落下のサンプルの数 ``broadside_runs``、公称の風 ``wind_model``、``wind_speed``、``wind_direction_deg``、``wind_scale``、``wind_at_ref_height``\ （いずれも風の格子の ``dispersion_meta.json`` と同じ）である。
モンテカルロの風速と風向の揺らぎは、この公称の風の周りに引く。
``descents`` の各要素は、降下モード ``descent``、着地したサンプル数 ``n``、失敗数 ``failed``、そのうち着地しなかった数 ``not_landed``、着地点の平均と共分散行列、
:math:`1\sigma` と :math:`3\sigma` の誤差楕円（長半径、短半径、長軸の方位）、最大着地距離 ``max_landing_distance`` とそのサンプル番号、
着地距離のパーセンタイル ``landing_distance_p50`` ``landing_distance_p90`` ``landing_distance_p99`` ``landing_distance_p999``\ （P99.9。着地したサンプルについて、線形補間で求める。着地したサンプルが 1000 未満の降下モードでは ``null``。:ref:`P99.9 の条件 <sec-mc-percentile>`）、頂点高度の平均と標準偏差を持つ（:ref:`sec-mc`）。
失敗したサンプルは、どの統計にも入らない。``failed`` が 1 以上のとき、``complete`` は ``false`` で、統計は着地したサンプルだけを表し、真のばらつきはこれより大きいか不明である。
同じとき ``max_landing_distance_is_lower_bound`` が ``true`` になり、最大着地距離は下限である。
着地点の平均は着地したサンプルが 1 つ以上、標準偏差・共分散・誤差楕円・頂点高度の標準偏差は 2 つ以上のとき求まる。サンプルが足りない統計は ``null`` である
（着地したサンプルが一つもなければ、統計と距離の項目はすべて ``null``）。
