.. _page-usage:

使い方
======

IgnisYeet は、コマンド ``ignisyeet`` にサブコマンドと設定ファイルを渡して使う。
1 回の解析は、``geom`` で形状を取り出し、``aero`` で係数表を作り、``sim`` で 1 回飛ばし、``dispersion`` で落下分散を求める順に進む。各コマンドは結果をファイルとして出力先に書き、端末には設定、進み具合、結果の要約を表示する。
結果が使えるかどうかは、端末の要約、終了ステータス、``summary.json`` の三つで判断できる。

.. _page-reference-cli:

コマンド
--------

``ignisyeet [-q] [--no-progress] <サブコマンド> <config> [オプション]`` の形で呼ぶ。``<config>`` は設定ファイルのパスである。

.. _sec-cli-subcommands:

サブコマンド
~~~~~~~~~~~~

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - コマンド
     - 内容と書き出すファイル
   * - ``geom <config>``
     - STL から形状を抽出する。``geometry.json`` と ``profile.csv`` を書く。
   * - ``aero <config> [--force]``
     - 空力係数表 ``aero_table.csv`` を作る。入力と設定が前回と同じなら、出力先にある表をそのまま使う。``--force`` を付けると再計算する（``aero.method = "table"`` では効果がない）。
   * - ``sim <config> [--descent ballistic|parachute]``
     - 設定した風で 1 回飛翔させる。``trajectory.csv`` と ``summary.json`` を書く。``--descent`` は ``sim.descent`` を上書きする。係数表がないか入力が変わったときは、先に ``aero`` を実行する。
   * - ``dispersion <config>``
     - ``dispersion.mode`` に従って落下分散を並列に計算する。``"wind_grid"`` は ``dispersion.csv`` と ``dispersion_meta.json`` を、``"monte_carlo"`` は ``dispersion_mc.csv`` と ``dispersion_summary.json`` を書く。係数表がないか入力が変わったときは、先に ``aero`` を実行する。
   * - ``cfd-check <config> [--mesh]``
     - CFD に必要なツール（SU2、MPI、gmsh。``surface = "stl"`` では manifold3d も）の有無と版、ケース数を表示する。``--mesh`` は壁面のメッシュも作って品質を表示する。``surface = "stl"`` では形状の忠実度、``surface = "step"`` では CAD のモデルの読み込みと位置合わせも表示する。必要なツールが足りないときは終了ステータス 1 で終わる。
   * - ``sample-stl <path>``
     - 付属のサンプル機体の STL（mm 単位、ノーズ +z 向き）を ``<path>`` に書く。

各コマンドは、実行の開始時に、出力先にある同じ種類の前回の結果を削除する。落下分散は同じモードの前回の結果だけを削除し、もう一方のモードの結果は残す。

共通のオプションは次のとおりである。

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - オプション
     - 内容
   * - ``-q`` ``--quiet`` ``--no-progress``
     - 出力の量と進捗表示の指定（:ref:`sec-ui-modes`）。
   * - ``--set KEY=VALUE``
     - ``geom`` ``aero`` ``sim`` ``dispersion``。設定のキーを上書きする。
   * - ``--out DIR``
     - ``geom`` ``aero`` ``sim`` ``dispersion``。出力先を ``output.dir`` の代わりに ``DIR`` にする。
   * - ``--aero-table PATH``
     - ``aero`` ``sim`` ``dispersion``。別の出力先で作った係数表を使う。
   * - ``--help`` ``--version``
     - 使い方、版を表示する。

.. _sec-cli-overrides:

設定の上書き
~~~~~~~~~~~~

設定ファイルを書き換えずに、コマンドラインで設定を変える。

``--set KEY=VALUE``
   設定のキー ``KEY`` を ``VALUE`` に変える。何度でも指定でき、指定した順に設定ファイルの内容へ上書きしてから、ファイルと同じ検査（未知のキー、型、範囲）を行う。

   .. list-table::
      :header-rows: 1
      :widths: 18 82

      * - 項目
        - 規則
      * - キー
        - TOML のキーを ``.`` でつないだもの（``wind.profile_mean_speed``、``aero.panel.body_axial``）。ない表は作る。
      * - 値
        - TOML の値として読む（``15.6``、``true``、``[5.0, 10.0]``、``"monte_carlo"``）。TOML の値として読めない語は文字列とみなすので、``wind.model=profile`` に引用符はいらない。数や真偽値に読める文字列は ``'"..."'`` のように引用符で囲む。値の中の相対パスは、設定ファイルのディレクトリを基準にする。
      * - 重複
        - 同じキーを 2 回以上指定すると、最後の値を使い、警告 :ref:`IY-C500` を出す。
      * - 検査のエラー
        - メッセージは ``error[IY-Cnnn]: invalid config <ファイル> with the --set overrides`` で始まる。型の誤りと未知のキーでは、原因の行が ``--set`` で始まる。範囲の誤りは、ファイルの場合と同じ、キーの名前を含むメッセージになる。
      * - 形の誤り
        - ``KEY=VALUE`` の形でない引数（:ref:`IY-C002`）と、キーに空の部分がある引数（:ref:`IY-C001`）は、コマンドラインの使い方の誤りとして、設定を読む前に終了ステータス 2 で止まる。表でない項目の下にキーを作ろうとするとき、整数が TOML の範囲（64 ビット符号付き）を超えるときは、終了ステータス 1 のエラーになる（:ref:`IY-C003`、:ref:`IY-C004`）。
      * - 削除
        - ``--set`` はキーを消せない。

``--out DIR``
   出力先を ``output.dir`` の代わりに ``DIR`` にする。相対パスは、設定ファイルのディレクトリではなく現在のディレクトリを基準にする。同じ出力先で 2 つの実行を同時に動かすことはできない（:ref:`IY-O031`、:ref:`sec-output-lock`）。

``--aero-table PATH``
   別の出力先で作った係数表 ``PATH``\ （``aero_table.csv``、またはそれを含むディレクトリ）を使う。
   表は、この設定と同じ機体と空力の設定で作ったものでなければならない（違えば :ref:`IY-O004` のエラーで止まる）。
   表、付随の JSON、同じディレクトリにある ``geometry.json`` ``profile.csv`` ``aero_drag.csv`` ``panel_cp.csv`` ``cfd_report.json`` ``cfd_cases.csv`` は出力先に写され、以後その出力先の保存済みの表として使われる。
   ``aero.method = "table"``\ （:ref:`IY-O002`）および ``aero --force``\ （:ref:`IY-O003`）とは併用できない。

たとえば、パネル法の係数表を 1 回作り、風だけを変えた飛翔に使い回す。

.. code-block:: sh

   ignisyeet aero examples/sample.toml --set aero.method=panel --out runs/aero_panel
   ignisyeet sim  examples/sample.toml --set aero.method=panel --aero-table runs/aero_panel \
       --set wind.speed=7 --set wind.direction_deg=90 --out runs/sim_panel_7ms

.. _sec-ui-modes:

端末の表示
----------

``ignisyeet`` は、標準出力が端末のとき、バナーに続いて設定パネル（Configuration）を表示する。入力ファイル、機体、射点、回収、空力、環境（地球・大気・風）、積分、出力先が項目ごとに並ぶので、単位や風向の入れ間違いに実行前に気づける（:numref:`fig-screen-sim-start`）。

.. _fig-screen-sim-start:

.. figure:: /_generated/screens/screen_sim_start.png
   :width: 100%
   :alt: sim の起動画面（バナーと設定パネル）

   ``sim`` の起動画面。水色の見出しで項目が分かれ、設定値は太字で示す。見るのは、機体の質量、射点、風が意図した値になっていることである。

計算が進むと、工程ごとに、実行中は進捗バーを、終わると完了行（チェック印と処理名、要約、所要時間）を表示する。

``aero``
   Barrowman 法では係数表の行数を分母にしたバーを出す。パネル法では段階ごとに進み具合を示し、2 行目に実行中の処理を表示する。前回と同じ設定の表を使うときは、バーを出さずに ``cached`` と行数を表示する（:numref:`fig-screen-aero-panel`）。

``sim``
   飛翔の段階（``rail`` ``powered`` ``coast`` ``parachute`` など）と、その時点の高度・速度・Mach 数・静安定余裕を表示し続ける。

``dispersion``
   終わったケース数のバーに、1 秒あたりの処理数、経過時間、残り時間の見積り（ETA）を表示する。2 行目に、ここまでの最大着地距離（弾道落下とパラシュート降下）が出る。
   分母は、風の格子では風速 × 風向 × 降下モードのケース数、モンテカルロ解析ではサンプル数 × 降下モードの数（パラシュートが有効なら 2）である（:numref:`fig-screen-dispersion`）。

すべての工程に緑のチェックが付いて終わるのが正常である。

.. _fig-screen-aero-panel:

.. figure:: /_generated/screens/screen_aero_panel.png
   :width: 100%
   :alt: aero のパネル法の実行中の画面

   ``aero``\ （``method = "panel"``）の実行中。形状の要約に続いて、段階の進み具合（4/9）と ETA、実行中の処理が出る。

.. _fig-screen-dispersion:

.. figure:: /_generated/screens/screen_dispersion.png
   :width: 100%
   :alt: dispersion（モンテカルロ）の実行中の画面

   ``dispersion``\ （``mode = "monte_carlo"``）の実行中。処理数、1 秒あたりの処理数、ETA、ここまでの最大着地距離が出る。処理数は、弾道落下とパラシュート降下を別々に数える。

表示の形は、標準出力と環境で次のように変わる。

.. list-table::
   :header-rows: 1
   :widths: 30 70

   * - 条件
     - 動作
   * - 標準出力が端末
     - バナー、パネル、色を使う表示にする。進捗バーは標準エラー出力が端末のときだけ出す。
   * - 標準出力をパイプ・リダイレクトした
     - プレーンモード。バナー、パネル、色、進捗バーを出さず、行単位の出力を書く。書き出したファイルは ``-> パス`` の行に並べる。
   * - ``-q`` ``--quiet``
     - エラー、警告、書き出したファイルのパスだけを表示する。警告は標準エラー出力に ``warning[IY-…]:`` の行で書く。
   * - ``--no-progress``
     - 進捗バーとスピナーを出さない。枠付きの要約は表示する。
   * - ``NO_COLOR`` が空でない値で設定されている
     - 色を使わない。枠や記号は残る。
   * - ``TERM=dumb``
     - 色と Unicode の記号を使わない。
   * - ロケールが UTF-8 でない（``LANG`` ``LC_ALL`` ``LC_CTYPE`` が UTF-8 を指さない、または ``TERM=linux``）
     - 枠線・記号・バーを ASCII 文字（``+ - |``、``ok`` ``x`` ``!``、``#>-``）に置き換える。

.. _page-howto-read-results:

結果を読む
----------

``sim`` は完了すると ``Flight result`` パネルを出す（:numref:`fig-screen-sim-result`）。見る行は次のとおりである。

.. list-table::
   :header-rows: 1
   :widths: 22 78

   * - 行
     - 見ること
   * - ``wind``
     - 計算に使った風（風速と、風が吹いてくる方位）。``profile`` では表の代表風になる。
   * - ``rail exit``
     - ランチャ離脱の時刻、速度、静安定余裕。離脱速度が 15 m/s を下回ると警告が出る。
   * - ``burnout``
     - 燃焼終了の時刻。飛翔が推力曲線の終わりより前に終わったときは ``not reached`` と出る。
   * - ``max speed``
     - 最大速度（Mach 数）と最大動圧。
   * - ``min margin``
     - 静安定余裕の最小値と、その時刻、Mach 数。判定の印が付く（下の表）。
   * - ``apogee``
     - 頂点の高度（射点からの高さ）と時刻。
   * - ``landing``
     - 着地点（射点からの東・北の距離、緯度・経度）、着地までの時間、着地速度。着地速度は地面に対する速さで、横方向の速さを含む。
   * - ``main deploy``
     - 二段開傘のとき、メインが開いた高度と時刻。

サンプルでは、頂点 4440.5 m、静安定余裕の最小値 1.99 cal（ランチャ離脱時）、着地点が東 −655.6 m、北 −7707.7 m、着地速度 7.1 m/s と出る。

.. _fig-screen-sim-result:

.. figure:: /_generated/screens/screen_sim_result.png
   :width: 100%
   :alt: sim の完了画面（結果パネルと出力ファイル）

   ``sim`` の完了画面。上から、工程の完了行、``Flight result`` パネル、警告、書き出したファイルの一覧、所要時間である。見るのは ``min margin`` の印と、パネルの下の警告の有無である。

静安定余裕には、次の判定の印が付く。

.. list-table::
   :header-rows: 1
   :widths: 28 32 40

   * - 静安定余裕
     - 表示
     - 意味
   * - 1.5 cal 以上
     - 緑のチェック印
     - 十分な余裕がある
   * - 1.0 cal 以上 1.5 cal 未満
     - 黄の警告記号と ``marginal (< 1.5 cal)``
     - 余裕が小さい。突風や重心のずれで不安定に近づく
   * - 0 cal 以上 1.0 cal 未満
     - 赤の記号と ``low static margin``
     - 余裕が不足している。設計を見直す
   * - 0 cal 未満
     - 赤のバツ印と ``UNSTABLE``
     - 不安定（圧力中心が重心より前にある）。結果は使えない

ここでの余裕は迎角 0 の極限の小迎角の余裕で、最小値を取る範囲とその定義は :ref:`sec-static-margin-general` にある。
サンプルでは燃焼中に重心が前へ移るので、最小はランチャ離脱の時刻にある。

終了ステータスが 3 のとき、標準エラー出力の最後の行が理由を示す。

.. list-table::
   :header-rows: 1
   :widths: 24 40 36

   * - 行の頭
     - 意味
     - 扱い
   * - ``INCOMPLETE RESULTS``
     - 結果は書いたが不完全。``sim.max_time`` までに着地しなかった、または失敗や着地しなかったケース、サンプルがある。
     - 理由を直して再実行する。着地しなかった飛翔の着地点は存在しない。
   * - ``RESULTS NOT USABLE``
     - 静的に不安定、静的な安定を判定できない、係数表の迎角または Mach 数の範囲を超えて係数を引いた、のいずれか。
     - 結果を判断に使わない。特に保安の判断に使わない。

不安定な機体の例として、サンプルの ``rocket.cg_dry`` を 1.3 m にすると、次のように出る。

.. code-block:: text

   warning[IY-O506]: static margin -1.49 cal at t = 5.96 s, Mach 0.03 is negative: the centre of pressure is forward of the CG, the rocket is UNSTABLE
   ignisyeet: RESULTS NOT USABLE: [IY-O020] the rocket is statically UNSTABLE (min static margin -1.49 cal at t = 5.96 s, Mach 0.03) (exit status 3)

``sim.max_time`` を 100 s にすると、サンプルは着地せず、次のように出る。

.. code-block:: text

   landing         NONE: the rocket did not reach the ground within sim.max_time = 100 s
   warning[IY-O504]: THE FLIGHT DID NOT LAND within sim.max_time = 100 s: there is no landing point; raise sim.max_time or check the descent (parachute, drag area, launch angle)
   ignisyeet: INCOMPLETE RESULTS: [IY-O027] the flight did not land within sim.max_time = 100 s (exit status 3)

``RESULTS NOT USABLE`` が静的な不安定によるときは、重心を前へ移すか、フィンを大きくして、静安定余裕を増やす。係数表の範囲外によるときは、``aero.alpha_max_deg`` や ``aero.mach_max`` を大きくして表を作り直す。
``INCOMPLETE RESULTS`` では、降下方式（パラシュートの有無と抗力面積）、``sim.max_time``、風の設定を見直す。

スクリプトから結果を使うときは、``summary.json`` の次の項目で機械的に判定する。

.. list-table::
   :header-rows: 1
   :widths: 28 72

   * - 項目
     - 見ること
   * - ``stable``
     - ``true`` なら静的に安定、``false`` なら不安定、``null`` なら判定できない。``true`` 以外は結果を使わない。
   * - ``landed``
     - ``false`` なら着地しなかった。着地に関する項目は ``null`` になる。
   * - ``aero_table_range``
     - ``angles_in_range`` と ``machs_in_range`` が ``true`` であること。``false`` なら、係数表の範囲外の係数を使った結果で、使えない。
   * - ``warnings``
     - 警告の配列。各文字列は ``[IY-O514] …`` のようにコードを角括弧で先頭に付ける。空でなければ、すべて読む。

サンプルでは、``stable`` が ``true``、``landed`` が ``true``、``angles_in_range`` と ``machs_in_range`` が ``true`` で、``warnings`` に ``[IY-O514] …`` の 1 件がある。
結果を使う前に、終了ステータスが 0 で、これらの項目が揃い、``warnings`` の各行の意味を理解していることを確かめる。
``dispersion`` の結果は、これに加えて、不安定なケースと着地しなかったケースの数を見る（:ref:`page-howto-dispersion-run`）。全項目は :ref:`sec-file-summary` にある。

.. _page-reference-exit-codes:

終了ステータス
--------------

.. _sec-exit-status:

終了ステータスの定義
~~~~~~~~~~~~~~~~~~~~

.. list-table::
   :header-rows: 1
   :widths: 10 40 50

   * - 値
     - 定義
     - 標準エラー出力の見出し
   * - 0
     - 成功。
     - なし
   * - 1
     - エラー。不正な設定、入力ファイルがない、処理の失敗、``cfd-check`` で必要なツールが足りない、など。
     - ``error[IY-…]: …``
   * - 2
     - コマンドラインの使い方の誤り。
     - ``error: …``
   * - 3
     - 出力は書いたが、結果が不完全、または使えない。
     - ``INCOMPLETE RESULTS`` または ``RESULTS NOT USABLE``

終了ステータスは、コマンドの直後に ``echo $?`` で確かめる。値ごとに読む行が決まる（:numref:`fig-exit-status-flow`）。

.. _fig-exit-status-flow:

.. figure:: /_generated/tikz/exit_status_flow.*
   :width: 100%
   :alt: 終了ステータスから見る場所への流れ図

   終了ステータスから、読む行を決める流れ。左から、成功（0）、エラー（1）、使い方の誤り（2）、結果が不完全または使えない（3）である。見るのは、値ごとに読む行が違うことで、1 と 3 は行頭のコードを一覧で引く。

終了ステータス 3 になる状況は次のとおりである。

.. list-table::
   :header-rows: 1
   :widths: 40 20 40

   * - 状況
     - 見出し
     - コード
   * - ``sim`` で ``sim.max_time`` までに着地しなかった。
     - ``INCOMPLETE RESULTS``
     - :ref:`IY-O027`
   * - ``dispersion`` の風の格子で、着地しなかった、または積分が失敗したケースがある。
     - ``INCOMPLETE RESULTS``
     - :ref:`IY-O028`
   * - ``dispersion`` のモンテカルロで、着地しなかった、または失敗したサンプルがある。
     - ``INCOMPLETE RESULTS``
     - :ref:`IY-O029`
   * - 静的に不安定な飛翔（``sim`` と、``dispersion`` の公称の飛翔）。
     - ``RESULTS NOT USABLE``
     - :ref:`IY-O020`
   * - 静的な安定を判定できない飛翔（``sim`` と、``dispersion`` の公称の飛翔）。
     - ``RESULTS NOT USABLE``
     - :ref:`IY-O021`
   * - ``dispersion`` に静的に不安定なケースやサンプルがある。
     - ``RESULTS NOT USABLE``
     - :ref:`IY-O025`
   * - ``dispersion`` に静的な安定を判定できないケースやサンプルがある。
     - ``RESULTS NOT USABLE``
     - :ref:`IY-O026`
   * - 係数表の Mach 数の範囲を超えて係数を引いた。
     - ``RESULTS NOT USABLE``
     - :ref:`IY-O022`
   * - 係数表の Mach 数の範囲より小さい側で係数を引いた。
     - ``RESULTS NOT USABLE``
     - :ref:`IY-O023`
   * - 係数表の迎角の範囲を超えて係数を引いた。
     - ``RESULTS NOT USABLE``
     - :ref:`IY-O024`

標準エラー出力に書く行は 1 つである。理由が重なったときは、次の順に優先する。

#. （公称の）飛翔が不安定か、判定できない。
#. 係数表の迎角の範囲外。
#. ケースやサンプルの不安定と、判定できないこと。
#. 着地しなかったこと。

係数表の Mach 数と迎角の範囲外が重なったときは、両方の文を ``;`` でつなぐ。不安定なケースと判定できないケースが両方あるときも、両方の節を ``;`` でつなぐ。

.. _sec-exit-definitions:

用語の定義
~~~~~~~~~~

用語は次のとおり定義する。

静的に不安定
   静安定余裕の最小値が 0 cal 未満である。最小値は、自由飛行中の最小値とランチャ離脱時の値の小さいほうで、結果パネルの ``min margin`` と同じである。
   余裕は迎角 0 の極限の小迎角の余裕である（:ref:`sec-static-margin-general`）。``trajectory.csv`` の ``stability_cal``\ （その状態の迎角での表示）が負になっても、それだけでは不安定としない。

静的な安定を判定できない
   ある面の法線力の傾きが 0 か、横流れと同じ向きの力（物理的でない符号）と復元するモーメントの組合せで、余裕を cal で表せない（一般表の各面と、軸対称表の :math:`C_{N\alpha}`）。
   求まった余裕に負のものがあれば不安定とし、そうでなく求まらない余裕があれば判定できないとする。どの時刻でも余裕を求めなかった飛翔（ランチャを離れなかった飛翔）も判定できない。ランチャを離れれば、離脱時の値は対気速度によらず求める。
   静的な安定は、安定、不安定、判定できないの 3 通りである。

係数表の範囲外
   係数を引く Mach 数または全迎角が、``aero_table.csv`` の格子の範囲を超えた（または下回った）引き出しが 1 回でもあること。
   迎角は、表の端の区間からの外挿になる。Mach 数は外挿せず、表の端の値で一定とする（:ref:`sec-aero-interp`）。
   飛翔計算は全迎角を :math:`\min(\alpha_t,\,180^\circ-\alpha_t)` に折り返して表を引くので（:ref:`sec-aero-table`）、軸対称の表では ``aero.alpha_max_deg = 90`` で迎角の範囲を超えない。

着地しなかった飛翔
   着地点が存在しない。``summary.json`` の ``landed`` が ``false`` になり、着地に関する項目は ``null`` になる（:ref:`sec-file-summary`）。

終了ステータス 3 のときに残る記録を次に示す。

.. list-table::
   :header-rows: 1
   :widths: 40 60

   * - 記録
     - 内容
   * - ``summary.json`` の ``stable`` と ``static_margin_min_cal``
     - 公称の飛翔の判定に使った値。判定できないときは ``stable`` が ``null`` になり、``stability_unknown`` に理由を書く。
   * - ``dispersion_meta.json`` と ``dispersion_summary.json`` の ``unstable`` と ``unstable_min_static_margin_cal``
     - 降下モードごとの不安定なケースの数と最小の余裕。
   * - 同 ``stability_unknown_runs``
     - 降下モードごとの、判定できないケースとサンプルの数。
   * - ``aero_table_range`` の ``angles_in_range`` と ``machs_in_range``
     - 係数表の範囲外の引き出しがあったとき ``false`` になる。
   * - ``ignisyeet.kml``
     - 着地しなかった結果は入れない。``sim`` が着地しなかったときは、飛翔の層を除いたファイルを書き直す（``output.kml = true`` のとき）。モンテカルロの誤差楕円は、着地したサンプルが 2 つ未満の降下モードでは入れない。

``dispersion`` は、不安定なケースとサンプルを降下モードごとに数え、統計には入れたままにする（:ref:`sec-dispersion-unstable`）。
``RESULTS NOT USABLE`` の結果は、保安の判断に使わない。

.. _page-reference-output-files:

出力ファイル
------------

出力先ディレクトリ（``output.dir``）に書かれるファイルを次に示す。
係数表と CFD の出力の列は :ref:`page-reference-aero-table-format` と :ref:`page-reference-cfd-output-files` にある。

.. _sec-output-list:

ファイルの一覧
~~~~~~~~~~~~~~

.. list-table::
   :header-rows: 1
   :widths: 30 20 50
   :class: longtable

   * - ファイル
     - 書くコマンド
     - 内容
   * - ``geometry.json``
     - ``geom`` ``aero``
     - 抽出した形状（:ref:`sec-file-geometry`）
   * - ``profile.csv``
     - ``geom`` ``aero``
     - 断面ごとの胴体半径とフィン高さ（:ref:`sec-file-geometry`）
   * - ``aero_table.csv`` ``aero_table.json``
     - ``aero``
     - 空力係数表と付随ファイル（:ref:`sec-aero-table-format`）
   * - ``aero_drag.csv``
     - ``aero``
     - ``method = "barrowman"`` と ``"panel"``。Mach 数ごとの抗力の内訳（:ref:`sec-file-drag`）
   * - ``panel_cp.csv``
     - ``aero``
     - ``method = "panel"``。表面パネルの圧力係数（:ref:`sec-file-panel-cp`）
   * - ``cfd_cases.csv`` ``cfd_report.json``
     - ``aero``
     - ``method = "cfd"``。ケースごとの結果と実行の要約（:ref:`sec-file-cfd`）
   * - ``<workdir>/``
     - ``aero``
     - ``method = "cfd"``。CFD の作業ディレクトリ（:ref:`sec-file-mesh`）
   * - ``trajectory.csv``
     - ``sim``
     - 飛翔の時刻歴（:ref:`sec-file-trajectory`）
   * - ``summary.json``
     - ``sim``
     - 1 回の飛翔の要約（:ref:`sec-file-summary`）
   * - ``dispersion.csv`` ``dispersion_meta.json``
     - ``dispersion``
     - ``mode = "wind_grid"``。風の各ケースの結果と付随情報（:ref:`sec-file-dispersion`）
   * - ``dispersion_mc.csv`` ``dispersion_summary.json``
     - ``dispersion``
     - ``mode = "monte_carlo"``。サンプルごとの結果と統計（:ref:`sec-file-dispersion`）
   * - ``ignisyeet.kml``
     - ``sim`` ``dispersion``
     - ``output.kml = true`` のとき。Google Earth 用の KML ファイル 1 つ（:ref:`sec-kml`）
   * - ``plots/*.png``
     - ``python/plot.py``
     - 図（:ref:`sec-plots`）

以下に、形状・飛翔・落下分散の出力の列と項目を示す。

.. _sec-file-geometry:

``geometry.json``
~~~~~~~~~~~~~~~~~

``geom`` と、係数表を作る ``aero`` が書く、抽出した形状である。
全長 ``length``、基準半径 ``ref_radius`` と基準面積 ``ref_area``、底面半径 ``base_radius``、ノーズ ``nose``、ボートテール ``boattail``、胴体のぬれ面積 ``wetted_area_body`` と体積 ``volume``、
フィンの平面形の面積 ``planform_area`` と重心位置 ``planform_centroid``、フィン ``fins``、STL の座標との対応 ``frame``\ （``axis_point``、``aft_direction``、``nose_offset``）、断面ごとの ``profile`` を持つ。
機体軸 :math:`x_b,y_b,z_b` を STL の座標で表した単位ベクトル ``body_x``、``body_y``、``body_z`` も書く（:ref:`sec-body-basis`）。
抽出の妥当性についての所見は、文字列の配列 ``warnings`` に書く（疑わしい点がなければ空。:ref:`sec-geometry-warnings`）。各文字列は ``[IY-G5nn] …`` の形で、コードを角括弧のまま先頭に付ける（:ref:`errors`）。

``profile.csv``
~~~~~~~~~~~~~~~

見出しは ``x,r,fin_span`` で、断面の位置 ``x``、胴体半径 ``r``、フィン高さ ``fin_span`` [m] を断面ごとに書く。

.. _sec-file-summary:

``summary.json``
~~~~~~~~~~~~~~~~

``sim`` が書く、1 回の飛翔の要約である。項目は次のとおりである。長さは m、速さは m/s、時刻は s、角度は deg である。

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - 項目
     - 内容
   * - ``descent``
     - 降下モード
   * - ``wind_speed`` / ``wind_direction_deg``
     - 基準風の風速 [m/s] と風向 [deg]。``profile`` では表の平均風速 :math:`W` と平均風の風向 :math:`\Psi`\ （:ref:`sec-wind-profile`）
   * - ``rail_exit_time`` / ``rail_exit_speed``
     - ランチャ離脱の時刻 [s] と速度 [m/s]（:ref:`sec-rail`）
   * - ``burnout_time``
     - 燃焼終了時刻 [s]（推力曲線の終わりの時刻）。飛翔がそれより前に終わったときは ``null``
   * - ``max_speed`` / ``max_mach`` / ``max_dyn_pressure``
     - 最大速度 [m/s]、最大 Mach 数、最大動圧 [Pa]
   * - ``apogee_time`` / ``apogee`` / ``apogee_east`` / ``apogee_north``
     - 頂点の時刻 [s]、射点からの高さ [m]、位置（東・北、射点から [m]）
   * - ``min_stability_cal``
     - 自由飛行の最小静安定余裕 [cal]。求める範囲と定義は :ref:`sec-static-margin-general` にある。該当する時刻がなければ ``null``
   * - ``stability_at_rail_exit``
     - ランチャ離脱時の静安定余裕 [cal]。対気速度によらず求める。軸対称表では :math:`(x_{cp}(\delta)-x_{cg})/d`\ （:eq:`eq-margin-axisym`）、一般表ではピッチとヨーの余裕の小さいほう
   * - ``landing_time``
     - 着地の時刻 [s]（:ref:`sec-landing`）
   * - ``landing_east`` / ``landing_north`` / ``landing_lat`` / ``landing_lon``
     - 着地位置。東・北は射点から [m]、緯度・経度は [deg]
   * - ``landing_distance``
     - 射点からの距離 [m]
   * - ``landing_speed``
     - 着地の瞬間の地面に対する速さ [m/s]。水平方向の流され（風による移動）の速さを含む。降下率（鉛直の速さ）ではなく、パラシュート降下で風が強いと降下率より大きい
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
     - 静的に安定なら ``true``、不安定なら ``false``、判定できないなら ``null``\ （定義は :ref:`sec-exit-definitions`）。
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
       最初の交差の時刻 ``t`` [s]、Mach 数 ``mach``、そのときのロール角速度（＝固有振動数）``frequency`` [rad/s]、上昇中の交差の回数 ``crossings`` を持つ。警告も出す（:ref:`IY-O510`）
   * - ``drogue_deploy_speed`` / ``drogue_deploy_q``
     - パラシュート（二段開傘ではドローグ）が開いたときの対気速度 [m/s]\ （風に対する速さ）と動圧 [Pa]。``drogue_deploy_time`` と同じく、開傘しなかった飛翔では書かない。
       対気速度が 50 m/s を超えるか動圧が 2 kPa を超えると警告する（:ref:`IY-O518`）
   * - ``main_deploy_speed`` / ``main_deploy_q``
     - 二段開傘のメインが開いたときの対気速度 [m/s] と動圧 [Pa]。メインが開かなかった飛翔では書かない
   * - ``earth_model``
     - 地球のモデル ``earth.model``\ （``"flat"`` または ``"ecef"``）
   * - ``wind_model``
     - 風のモデル ``wind.model``\ （``"constant"``、``"power"``、``"log"``、``"profile"``）
   * - ``wind_scale``
     - ``wind.model = "profile"`` のときだけ書く。その飛翔で表の風速に掛けた倍率 :math:`k`\ （CSV の表そのままなら 1。風の格子とモンテカルロのケースは、そのケースの平均風速に合わせた倍率。:ref:`sec-wind-scaling`）
   * - ``wind_at_ref_height``
     - ``wind.model = "profile"`` のときだけ書く。公称の表の風を ``wind.ref_height`` で求めた値で、高さ ``height`` [m]、風速 ``speed`` [m/s]、吹いてくる方位 ``direction_deg`` [deg] を持つ（無風なら方位は 0）。
       保安距離での使い方は :ref:`sec-dispersion-safety` にある
   * - ``max_time``
     - 打ち切り時刻 ``sim.max_time`` [s]
   * - ``config_hash``
     - どの設定で計算した結果かを示す識別子。現在の設定の結果かどうかの照合に使い、KML は現在の設定の結果だけをまとめる（:ref:`sec-kml`）。落下分散の出力にある ``config_hash`` と ``flight_config_hash`` も同じ目的の識別子である
   * - ``warnings``
     - 警告の文字列の配列（空のこともある）。各文字列は、画面の ``warning[IY-O514]: …`` の形ではなく、コードを角括弧のまま先頭に付けた ``[IY-O514] …`` の形で書く（:ref:`errors`）。設定の読み込み時の警告（``profile`` で使わない ``wind.speed`` と ``wind.direction_deg``、2 回以上指定した ``--set`` のキー）が先頭で、計算中の警告が続く。``dispersion_meta.json`` と ``dispersion_summary.json`` の ``warnings`` も同じ順である
   * - ``aero_table_range``
     - 係数表の範囲外の引き出しの集計。全引き出しの回数 ``lookups``、Mach 数が表の最小値未満・最大値超の回数 ``mach_below`` ``mach_above``、迎角または横滑り角が格子の外の回数 ``angle_beyond``、要求された最大の Mach 数 ``max_mach`` と最大の迎角 ``max_abs_alpha_deg`` [deg]（軸対称表では引いた全迎角、一般表では :math:`\alpha` と :math:`\beta` から求めた全迎角 :math:`\arccos(\cos\alpha\cos\beta)`）、範囲外が 1 回でもあったか ``out_of_range``\ （真偽値）、迎角の範囲外がなかったか ``angles_in_range`` と Mach 数の範囲外がなかったか ``machs_in_range``\ （真偽値。どちらかが ``false`` のときは結果を使えず、終了ステータス 3 で終わる。:ref:`sec-exit-status`）

``sim`` の実行が失敗したとき（積分の発散、``launch.elevation_deg`` が範囲外、ランチャを離れないなど）は、
``summary.json`` を失敗の印 ``{"failed": true, "error": "…"}`` だけで書き、``error`` にエラーの文を入れる。``trajectory.csv`` は書かない。``ignisyeet.kml`` があれば、飛翔の層を除いて書き直す（:ref:`sec-kml`）。
``python/plot.py`` は、この印があるとき飛翔経路を描かずにエラーで終わる。

.. _sec-file-trajectory:

``trajectory.csv``
~~~~~~~~~~~~~~~~~~

見出しは次のとおりである。

.. code-block:: text

   t,phase,east,north,up,lat,lon,alt,vel_e,vel_n,vel_u,airspeed,mach,alpha_deg,thrust,mass,x_cg,x_cp,stability_cal,dyn_pressure,cn,ca,pitch_deg,heading_deg,wx,wy,wz,beta_deg,roll_rate,cl,cy

時刻歴である。座標は射点を原点とする ENU（東・北・上）で、機体座標は機首方向を x 軸とする。

.. list-table::
   :header-rows: 1
   :widths: 20 12 68

   * - 列
     - 単位
     - 内容
   * - ``t``
     - s
     - 点火からの時刻
   * - ``phase``
     - 
     - ``rail``\ （ランチャ上）、``free``\ （自由飛行）、``parachute``\ （パラシュート降下）のいずれか
   * - ``east`` ``north`` ``up``
     - m
     - 射点からの位置。``up`` は上向きが正
   * - ``lat`` ``lon``
     - deg
     - 緯度と経度
   * - ``alt``
     - m
     - 標高（射点では ``launch.altitude``）
   * - ``vel_e`` ``vel_n`` ``vel_u``
     - m/s
     - 地面に対する速度の東・北・上成分。``vel_u`` は上昇が正
   * - ``airspeed``
     - m/s
     - 風に対する速さ
   * - ``mach``
     - 
     - Mach 数
   * - ``alpha_deg``
     - deg
     - 全迎角 :math:`\alpha_t`
   * - ``thrust``
     - N
     - 推力
   * - ``mass``
     - kg
     - その時刻の質量
   * - ``x_cg``
     - m
     - 重心。ノーズ先端から後方へ測る
   * - ``x_cp``
     - m
     - 圧力中心。ノーズ先端から後方へ測る。その状態が飛んでいる迎角での値で、一般表では等価な圧力中心（:eq:`eq-xcp-eff`）
   * - ``stability_cal``
     - cal
     - :math:`(x_{cp}-x_{cg})/d`。その状態の迎角での表示で、正が重心より後ろに圧力中心がある向きである。一般表ではこの値は ``x_cp`` から求める。迎角が大きいと圧力中心が前に動くので、ランチャ離脱の直後や頂点付近では小さくなる。静的な安定の判定と ``summary.json`` の余裕は迎角 0 の極限の小迎角の余裕で、この列とは違う（:ref:`sec-static-margin-general`）
   * - ``dyn_pressure``
     - Pa
     - 動圧
   * - ``cn`` ``ca``
     - 
     - 法線力係数と軸力係数。``ca`` は抗力の向きを正とする。一般表の ``cn`` は :math:`C_Y` と :math:`C_Z` の合成
   * - ``pitch_deg``
     - deg
     - 機体軸の水平面に対する仰角。上向きが正で、鉛直上向きが 90
   * - ``heading_deg``
     - deg
     - 機体軸の水平成分の方位。北が 0 で東回りに 0 以上 360 未満
   * - ``wx`` ``wy`` ``wz``
     - rad/s
     - 機体座標の角速度成分（右手系の正）。``earth.model = "ecef"`` では ECEF に対する角速度
   * - ``beta_deg``
     - deg
     - 横滑り角 :math:`\beta`
   * - ``roll_rate``
     - rad/s
     - ロール角速度 :math:`\omega_x`（``wx`` と同じ値）
   * - ``cl``
     - 
     - 適用したロールの係数 :math:`C_l^{\mathrm{tot}}`\ （:eq:`eq-cl-total`）
   * - ``cy``
     - 
     - 横力係数（機体座標の y）。軸対称表では 0

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

``mode = "wind_grid"`` が ``dispersion.csv`` と並べて書く。

.. list-table::
   :header-rows: 1
   :widths: 28 72

   * - 項目
     - 内容
   * - ``config_hash``
     - 風の格子の結果を計算した設定を示す識別子
   * - ``flight_config_hash``
     - 飛翔の設定を示す識別子
   * - ``cases``
     - ケース数
   * - ``failed`` / ``errored``
     - 着地しなかったケースの数と、そのうち積分がエラーで止まったケースの数
   * - ``warnings``
     - 警告の文字列の配列
   * - ``aero_table_range``
     - 係数表の範囲外の引き出しの集計。項目は :ref:`sec-file-summary` と同じで、エラーで止まったケースは数えない
   * - ``static_margin_min_cal`` / ``stable``
     - 公称の飛翔の最小の静安定余裕 [cal] と、それが 0 以上かを示す真偽値（公称の飛翔は、設定した風での弾道落下）。公称の飛翔が失敗して余裕が求まらないときと、静的な安定を判定できないときは、``stable`` が ``null`` である
   * - ``unstable`` / ``unstable_min_static_margin_cal``
     - 降下モード（``ballistic`` と ``parachute``）ごとの、静的に不安定なケースの数と、そのモードのケースの最小の静安定余裕 [cal]。余裕が求まるケースがなければ ``null``
   * - ``stability_unknown_runs`` / ``stability_unknown``
     - 降下モードごとの、静的な安定を判定できないケースの数と、公称の飛翔が判定できないときの理由（判定できたときは ``null``）
   * - ``broadside_runs``
     - 横向きに落ちた（``broadside_time`` が 10 s を超えた）ケースの数。弾道落下だけを数え、``{"ballistic": 1}`` の形
   * - ``wind_model``
     - 公称の風のモデル
   * - ``wind_speed`` / ``wind_direction_deg``
     - 公称の基準風速と基準風向。``profile`` では表の平均風速 :math:`W` と平均風の風向 :math:`\Psi`\ （``profile_mean_speed`` と ``profile_mean_direction_deg`` を指定したときはその値）
   * - ``wind_scale`` / ``wind_at_ref_height``
     - ``profile`` のときだけ書く。公称の表の倍率（CSV の風速に掛ける倍率。格子の風速はここからさらに拡大する。:ref:`sec-wind-scaling`）と、``wind.ref_height`` での公称の表の風（``summary.json`` と同じ）

``stable`` が ``false`` か ``null`` のとき（公称の飛翔が失敗した場合を除く）、``unstable`` または ``stability_unknown_runs`` が 1 以上のモードがあるとき、迎角または Mach 数の範囲外の引き出しがあったとき（``angles_in_range`` または ``machs_in_range`` が ``false``）、``dispersion`` は終了ステータス 3 で終わる（:ref:`sec-exit-status`、:ref:`sec-dispersion-unstable`）。

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

``mode = "monte_carlo"`` が書く。全体の項目は次のとおりである。

.. list-table::
   :header-rows: 1
   :widths: 28 72

   * - 項目
     - 内容
   * - ``samples`` / ``seed``
     - サンプル数と乱数の種
   * - ``truncate_sigma``
     - 標準正規乱数を切る範囲（``dispersion.monte_carlo.truncate_sigma`` の値。切らないときは ``null``。:ref:`sec-mc`）
   * - ``descents``
     - 降下モードごとの配列（下表）
   * - ``config_hash`` / ``flight_config_hash``
     - どの設定で計算した結果かを示す識別子
   * - ``warnings`` / ``aero_table_range``
     - 警告の配列と、係数表の範囲外の引き出しの集計（項目は :ref:`sec-file-summary` と同じ。失敗したサンプルは数えない）
   * - ``static_margin_min_cal`` / ``stable``
     - 公称の飛翔の最小の静安定余裕 [cal] とその判定。``dispersion_meta.json`` と同じで、公称の飛翔が失敗したときは ``null``
   * - ``unstable`` / ``unstable_min_static_margin_cal``
     - 降下モードごとの、静的に不安定なサンプルの数と最小の静安定余裕。``dispersion_meta.json`` と同じで、失敗したサンプルは余裕を持たないので数に入らない
   * - ``stability_unknown_runs`` / ``stability_unknown`` / ``broadside_runs``
     - ``dispersion_meta.json`` と同じ
   * - ``wind_model`` / ``wind_speed`` / ``wind_direction_deg`` / ``wind_scale`` / ``wind_at_ref_height``
     - 公称の風。``dispersion_meta.json`` と同じで、モンテカルロの風速と風向の揺らぎはこの公称の風の周りに引く

``descents`` の各要素の項目は次のとおりである。

.. list-table::
   :header-rows: 1
   :widths: 28 72

   * - 項目
     - 内容
   * - ``descent``
     - 降下モード
   * - ``n`` / ``failed`` / ``not_landed``
     - 着地したサンプル数、失敗数、そのうち着地しなかった数
   * - 着地点の平均と共分散行列
     - 着地点の統計
   * - 誤差楕円
     - :math:`1\sigma` と :math:`3\sigma` の長半径、短半径、長軸の方位
   * - ``max_landing_distance``
     - 最大着地距離とそのサンプル番号
   * - ``landing_distance_p50`` ``_p90`` ``_p99`` ``_p999``
     - 着地距離のパーセンタイル。着地したサンプルについて線形補間で求める。P99.9 は、着地したサンプルが 1000 未満の降下モードでは ``null``\ （:ref:`P99.9 の条件 <sec-mc-percentile>`）
   * - 頂点高度
     - 平均と標準偏差（:ref:`sec-mc`）

失敗したサンプルは、どの統計にも入らない。``failed`` が 1 以上のとき、``complete`` は ``false`` で、統計は着地したサンプルだけを表し、真のばらつきはこれより大きいか不明である。
同じとき ``max_landing_distance_is_lower_bound`` が ``true`` になり、最大着地距離は下限である。
着地点の平均は着地したサンプルが 1 つ以上、標準偏差・共分散・誤差楕円・頂点高度の標準偏差は 2 つ以上のとき求まる。サンプルが足りない統計は ``null`` である
（着地したサンプルが一つもなければ、統計と距離の項目はすべて ``null``）。

.. _page-howto-plots-kml:

図と KML
--------

作図コマンド ``ignisyeet-plot`` は計算結果の図を描く。``ignisyeet.kml`` を Google Earth で開くと、飛翔経路と着地点を地図の上で確かめられる。

.. _sec-cli-plot:

作図コマンド
~~~~~~~~~~~~

インストーラで入れたときは ``ignisyeet-plot`` が使える。``pip`` で作る方法は :ref:`sec-python-env` にある。リポジトリから使うときは ``python/plot.py``\ （パッケージ名 ``ignisyeet-plot``）を ``plot.py <図> <出力ディレクトリ> [--compare DIR]`` の形で呼ぶ。
``ignisyeet-plot all out`` は、``out/plots/`` に、元のデータがある図の PNG を書く。サンプルを ``sim`` だけ実行したときは次のとおりである。

.. code-block:: text

   wrote out/plots/geometry.png
   wrote out/plots/aero.png
   skip panel: panel_cp.csv not found
   wrote out/plots/trajectory_3d.png
   wrote out/plots/trajectory.png
   skip dispersion: dispersion.csv / dispersion_mc.csv not found

``all`` は、元のデータがない図を ``skip …`` と表示して飛ばす。描く図を 1 つに絞るには、図の名前を指定する（``ignisyeet-plot trajectory out``）。

.. list-table::
   :header-rows: 1
   :widths: 18 30 52

   * - 図
     - 元のデータ
     - 描く図
   * - ``all``
     - 出力ディレクトリにある結果
     - 描けるすべての図を ``<出力ディレクトリ>/plots/`` に書く。
   * - ``geometry``
     - ``profile.csv``
     - ``geometry.png``：胴体半径とフィンの高さ（抽出結果の確認）
   * - ``aero``
     - ``aero_table.csv``、``aero_drag.csv``
     - ``aero.png``：迎角 0 の抗力係数 :math:`C_{D0}` の内訳、法線力係数の傾き :math:`C_{N\alpha}`、圧力中心 :math:`x_{cp}`、法線力係数と迎角
   * - ``panel``
     - ``panel_cp.csv``
     - ``panel.png``：パネル法の表面の圧力係数（``method = "panel"`` のとき）
   * - ``trajectory``
     - ``trajectory.csv``、``summary.json``
     - ``trajectory.png``：高度、対気速度、Mach 数、迎角、圧力中心と重心の距離、動圧、推力、質量、ピッチ角、横滑り角、ロール角速度の時間履歴。``trajectory_3d.png``：飛翔経路の 3 次元表示
   * - ``dispersion``
     - ``dispersion.csv``、``dispersion_meta.json``\ （風の格子）、``dispersion_mc.csv``、``dispersion_summary.json``\ （モンテカルロ）
     - ``dispersion.png``：風速ごとの着地点の輪郭。``dispersion_mc.png``：モンテカルロの着地点と誤差楕円

2 つの係数表を比べるときは、``aero`` に ``--compare`` を付ける（``ignisyeet-plot aero out_barrowman --compare out_panel``）。``aero_compare.png`` に、2 つの表の :math:`C_{N\alpha}`、:math:`x_{cp}`、:math:`C_{D0}` が Mach 数に対して重なって描かれる。凡例はディレクトリ名である。

.. _sec-plots:

図の内容
~~~~~~~~

結果が使えない、不完全、または注意が要るとき、飛翔経路の図と落下分散の図の上に、目立つ帯を描く。問題のない結果では帯が出ない。
帯の見出しは、``summary.json``\ （飛翔経路）、``dispersion_meta.json``\ （風の格子）、``dispersion_summary.json``\ （モンテカルロ）の項目から決まり、理由を 1 行ずつ並べる。

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - 見出し（帯の色）
     - 条件
   * - ``RESULTS NOT USABLE``\ （赤）
     - 静的に不安定（``stable`` が ``false``）、静的な安定を判定できない（``stable`` が ``null``）、係数表の迎角の範囲外の引き出しがある（``aero_table_range`` の ``angles_in_range`` が ``false``）、Mach 数の範囲外の引き出しがある（``machs_in_range`` が ``false``）。落下分散では、公称の飛翔のほか、不安定または判定できない飛翔を含む降下モードも数える
   * - ``INCOMPLETE``\ （橙）
     - 飛翔が着地しなかった（``landed`` が ``false``）、失敗または着地しなかったケースやサンプルがある
   * - ``CAUTION``\ （橙）
     - 弾道落下が横向きに落ちた（``fell_broadside`` が ``true``、または ``broadside_runs`` が 1 以上）。``large-angle aerodynamics, not validated`` と書く

条件が重なるときは、最も重い見出し（``RESULTS NOT USABLE``、``INCOMPLETE``、``CAUTION`` の順）を出し、理由はすべて並べる。
これらの項目を持たない JSON では帯を描かない。

落下分散の図では、次の着地点に印を重ねる。印の凡例には数を添える。

* 静的に不安定な飛翔：赤のバツ印（``statically unstable``）。
* 静的な安定を判定できない飛翔：橙の白抜きの三角（``stability unknown``）。
* 横向きに落ちた弾道落下：紫の白抜きのひし形（``fell broadside``）。

風の格子の図では、風速ごとの線は風速の大きい順に並ぶ凡例とともに描く。最も大きい風速の線が最も外側とは限らない（弾道落下では、途中の風速で最も遠くなることがある）。
``wind.model = "profile"`` の結果では、凡例の見出しが ``profile mean speed, highest first`` になる。
横向きに落ちたケースのところで、風向の順に結ぶ線を切る。
モンテカルロの図には、最大着地距離と、着地距離の P50 / P90 / P99 / P99.9 を、枠で囲んで軸の下に書き添える。着地したサンプルが 1000 未満のときは P99.9 を書かず、``(P99.9 needs ≥ 1000 landed samples)`` と添える。

飛翔経路の時間履歴の図では、高度の欄だけが着地までの全体を描く。ほかの欄は、パラシュートの開傘まで（弾道落下では着地まで）である。
推力の欄は燃焼終了の時刻の 1.2 倍まで、圧力中心と重心の距離の欄は、ランチャを離れてから頂点までの上昇中を描く。
この欄は、その時刻の迎角での圧力中心と重心の距離 [cal] で、静的な安定の判定に使う小迎角の余裕とは別の量である（:ref:`page-howto-read-results`）。
二段開傘の飛翔では、軌跡の図にメインの開傘の位置を四角の印で示す。
飛翔経路の図の高度は、``summary.json`` の頂点高度と同じ定義の、射点からの高さである。

.. _sec-run-troubleshooting:

作図のメッセージ
~~~~~~~~~~~~~~~~

作図が表示するメッセージと意味は次のとおりである。

.. list-table::
   :header-rows: 1
   :widths: 46 54

   * - メッセージ
     - 意味と対処
   * - ``error: output directory … not found; run the ignisyeet commands first``
     - 出力先がない。先に ``ignisyeet`` で計算するか、パスを直す。終了ステータス 1。
   * - ``error: nothing to plot in …``
     - 描ける図が 1 つもない。計算を実行してから描く。終了ステータス 1。個別の図（``geometry`` など）を選んで何も描けなかったときも、``nothing to plot for …`` のエラーで終了ステータス 1 になる。
   * - ``skip geometry: profile.csv not found (run `ignisyeet geom` again)``
     - 形状の図の元がない。``ignisyeet geom`` を実行する。
   * - ``skip trajectory: the last sim run failed: …; not plotting the old trajectory``
     - 直前の ``sim`` が失敗している（``summary.json`` が失敗の印）。古い結果を描かない。失敗の原因を直して ``sim`` を実行し直す。``trajectory`` を直接指定したときは、``error: the last sim run failed: …`` で終了ステータス 1 になる。
   * - ``skip dispersion_mc: dispersion_summary.json not found (run `ignisyeet dispersion` again)``
     - モンテカルロの図の元の一部がない。``ignisyeet dispersion`` を実行し直す。
   * - ``skip panel: panel_cp.csv not found``
     - パネル法の結果がない。``aero.method = "panel"`` で計算したときだけ出る図なので、通常は無視してよい。
   * - ``removed … (plot skipped; the file was from an earlier run)``
     - 今回は描かなかった図の古い PNG が ``plots/`` に残っていたため、取り違えないよう消した。

.. _sec-kml:

Google Earth で見る（KML）
~~~~~~~~~~~~~~~~~~~~~~~~~~

``[output]`` の ``kml`` が ``true``\ （既定）のとき、``sim`` と ``dispersion`` は ``ignisyeet.kml`` を、CSV や JSON と同じ出力先に書き出す。
KML は地理情報の XML 形式で、OGC の標準（KML 2.2 :cite:`ogckml`）である。
Google Earth（デスクトップ版・Web 版）や QGIS で開くと、機体の経路や着地点を衛星写真の上に重ねて確認できる。
不要なときは ``kml = false`` にする。各要素をクリックすると、時刻・速度・着地点などの数値が説明欄に出る（説明欄の文は英語である）。
Google Earth の Web 版ではファイルの読み込みから、デスクトップ版ではファイルのダブルクリックで開く。

左のパネルに、次のフォルダが並び、チェックボックスで表示を切り替えられる。該当する結果がなければ、そのフォルダは作られない。

.. list-table::
   :header-rows: 1
   :widths: 28 72

   * - フォルダ
     - 内容
   * - ``Launch site``
     - 射点を示す白いピン。説明欄に射角、方位、レール長が載る。
   * - ``Flight``
     - ``sim`` の飛翔。経路を段階ごとに色分けする（黄がランチャ滑走、赤が燃焼中、青が燃焼後の慣性飛行、緑がパラシュート降下）。イベントの印は、ランチャ離脱（静安定余裕を含む）、燃焼終了、頂点、開傘、着地である。
   * - ``Dispersion – wind grid``
     - 風の格子の結果。降下モード（``ballistic`` と ``parachute``）ごとに、風速ごとの輪郭（同じ風速で風向を変えた着地点を風向の順に結んだ閉じた線）と、各ケースの着地点がある。弾道落下は青、パラシュート降下は橙で、風速が大きいほど濃い。
   * - ``Dispersion – Monte Carlo``
     - モンテカルロの結果。降下モードごとに、平均着地点、:math:`1\sigma` と :math:`3\sigma` の誤差楕円、各サンプルの着地点がある。

``sim`` と ``dispersion`` は、実行のたびに ``ignisyeet.kml`` を書き直し、同じ出力先にある他の結果も読み戻して 1 つにまとめる。
したがって、``sim`` のあとに ``dispersion`` を実行すると、``Flight`` と ``Dispersion`` のフォルダが 1 つの KML にそろい、飛翔経路と落下分散を同じ画面で重ねて見られる。
設定を変えたときは、``sim`` と ``dispersion`` の **両方** を実行し直す。現在の設定と一致しない前の結果は、古いものとして KML に入れず、警告を出す（:ref:`IY-O539`）。

着地しなかった飛翔と、着地しなかった風のケースは KML に入れない。モンテカルロでは、着地したサンプルだけを入れる。誤差楕円は、着地したサンプルが 2 つ未満の降下モードでは入れない。
``sim`` がエラーで止まったときは、前の飛翔が残らないよう、飛翔の層を除いて書き直す。

飛翔経路の高度は絶対高度（WGS84 楕円体からの高さ）で書き出す。射点で ``launch.altitude`` から始まり、飛翔中は ``launch.altitude`` に射点からの上向きの高さを足した値である。
KML の絶対高度は名目上は平均海面（EGM96 ジオイド）からの高度だが、IgnisYeet はジオイド高（楕円体と平均海面の差）を **扱わない**。場所によっては数十 m の差になる。
``launch.altitude`` には射点の **海抜標高** を入れ、地形の高さと合っていることを確かめる。合っていないと、経路が地面にめり込んだり浮いたりして見える。
射点・着地点・誤差楕円など地面に固定する図形は、Google Earth の地形に沿って表示される。射点のピンが地表に立ち、飛翔経路が地面から始まっていれば、高度は合っている。

.. _sec-cli-make:

make のターゲットと変数
-----------------------

リポジトリ直下の ``Makefile`` は、ビルド、実行、作図を呼ぶ。

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - ターゲット
     - 内容
   * - ``all``
     - ``aero`` ``sim`` ``dispersion`` ``plot`` を順に実行する。``sim`` か ``dispersion`` が終了ステータス 3 で終わっても作図まで進み、最後に ``make: results are INCOMPLETE or NOT USABLE (exit status 3): see the lines above`` を出して失敗する（:ref:`sec-exit-status`）。ほかの失敗ではその場で止まる。
   * - ``geom`` ``aero`` ``sim`` ``dispersion``
     - ビルドしてから同名のサブコマンドを ``CONFIG`` で実行する。
   * - ``plot``
     - ``python/plot.py all $(OUT)`` を実行する。
   * - ``build`` ``test`` ``clean``
     - ``cargo build --release``、``cargo test --release``、``cargo clean`` と ``$(OUT)`` の削除。

.. list-table::
   :header-rows: 1
   :widths: 18 30 52

   * - 変数
     - 既定値
     - 内容
   * - ``CONFIG``
     - ``examples/sample.toml``
     - 設定ファイル。
   * - ``OUT``
     - 設定ファイルの ``[output] dir``
     - 作図が読む出力ディレクトリ。``dir`` は設定ファイルのあるディレクトリを基準にする。省略時は設定ファイルのある場所の ``out``。
   * - ``BIN``
     - ``$(CARGO_TARGET_DIR)/release/ignisyeet``（未設定なら ``target/release/ignisyeet``）
     - 実行ファイル。
   * - ``PLOT``
     - ``uv run --project python python/plot.py``
     - 作図コマンド。

.. _page-howto-warnings-errors:

エラーとワーニングの表示
------------------------

エラーは ``error[IY-C062]: …``、ワーニングは ``warning[IY-O515]: …`` のように、コードを先頭に付けて表示する（リッチモードでは黄色の ``⚠ [IY-O515] …`` の行）。
エラーは処理を止め、ワーニングは処理を続ける。ワーニングは ``summary.json`` の ``warnings`` にも同じ文が残るので、後から結果のファイルだけを見るときはこの配列を読む。
コードの体系、表示の形、コードの探し方は付録の :ref:`errors` に、各コードの意味と対処は領域ごとの一覧にある。

.. _tbl-warnings-symptoms:

症状別の対処表
~~~~~~~~~~~~~~

コードが複数出ているときや、コードが分からないときは、近い症状を探す。

.. list-table::
   :header-rows: 1
   :widths: 24 28 48

   * - 症状
     - コード
     - 確かめる設定と、読むページ
   * - 設定ファイルが読めない、値が範囲外
     - :ref:`IY-C060`、``IY-C061``、``IY-C005``、``IY-C009`` から ``IY-C059``
     - メッセージが名指しする項目を直す。``--set`` の綴りの誤りは、未知のキーとして原因の行に出る。値の範囲の一覧は :ref:`sec-config-checks` にある。
   * - STL が読めない、表面が閉じていない
     - :ref:`IY-G001` から ``IY-G011``
     - STL を修復するか、ソリッドとして書き出し直す（:ref:`page-howto-own-rocket`）。
   * - 形状の抽出結果が疑わしい（比、向き、フィンの位置）
     - :ref:`IY-G501` から ``IY-G512``
     - ``rocket.stl_scale``、``rocket.nose_direction``、``rocket.fin_override`` を確かめる（:ref:`sec-geometry-warnings`）。
   * - 全長が範囲外、モータが機体に収まらない
     - :ref:`IY-O005` から ``IY-O008``、``IY-O502``、``IY-O503``
     - ``rocket.stl_scale``、``rocket.cg_dry``、``motor.aft_x`` を確かめる。
   * - 推力曲線のファイルが読めない、疑わしい
     - :ref:`IY-S001` から ``IY-S015``、``IY-S500`` から ``IY-S502``、``IY-O013``
     - ``.eng`` の行を直す（:ref:`page-howto-motor`）。
   * - レールを離れない
     - :ref:`IY-S080`
     - 推力、質量、``launch.rail_length`` を確かめる（:ref:`page-howto-launcher-parachute`）。
   * - ランチャ離脱速度が低い、風による迎角が大きい
     - :ref:`IY-O515`、``IY-O516``
     - ``launch.rail_length`` を長くする（:ref:`page-howto-launcher-parachute`）。
   * - 静的に不安定、または判定できない
     - :ref:`IY-O020`、``IY-O021``、``IY-O025``、``IY-O026``、``IY-O505`` から ``IY-O508``
     - 終了ステータス 3。``rocket.cg_dry`` とフィン、モータの取り付け位置を見直す。結果は使わない（:ref:`page-howto-read-results`）。
   * - 着地しない
     - :ref:`IY-O027`、``IY-O028``、``IY-O029``、``IY-O504``、``IY-O530``
     - 終了ステータス 3。降下方式、``recovery.cd_s``、``sim.max_time`` を確かめる（:ref:`page-howto-launcher-parachute`）。
   * - 係数表の範囲外の係数を使った
     - :ref:`IY-O022`、``IY-O023``、``IY-O024``
     - 終了ステータス 3。``aero.alpha_max_deg``、``aero.mach_max`` を大きくして表を作り直す。
   * - 弾道落下が横向きに落ちた
     - :ref:`IY-O509`、``IY-O536``
     - 終了ステータスは変わらない。着地点は検証していない大迎角の空力で決まる（:ref:`page-theory-limitations`）。
   * - パラシュートが高速で開く、着地速度が大きい、開傘が遅れた
     - :ref:`IY-O511`、``IY-O517`` から ``IY-O520``
     - ``recovery.delay``、``recovery.cd_s``、``recovery.main_altitude`` を確かめる（:ref:`page-howto-launcher-parachute`）。
   * - 風の表が読めない、表が飛翔の高度に足りない
     - :ref:`IY-S030` から ``IY-S037``、``IY-S050`` から ``IY-S053``、``IY-O512``、``IY-O513``、``IY-O521``
     - 表の行と高度の範囲を直す（:ref:`page-howto-environment`）。
   * - 頂点が高く、地上風のモデルが外挿になる
     - :ref:`IY-O514`
     - 高度 1 km を超える飛翔では、``wind.model = "profile"`` を使う（:ref:`page-howto-environment`）。
   * - 出力先を別の実行が使っている
     - :ref:`IY-O031`、``IY-O032``
     - ``--out`` で出力先を分ける（:ref:`sec-output-lock`）。
   * - ``--aero-table`` が受け付けられない
     - :ref:`IY-O002`、``IY-O003``、``IY-O004``
     - 表を同じ設定で作り直す（:ref:`page-howto-sweeps`）。
   * - 風の格子のケースやモンテカルロのサンプルが失敗した、統計が粗い
     - :ref:`IY-O011`、``IY-O029``、``IY-O531`` から ``IY-O533``、``IY-O537``、``IY-O538``
     - ``dispersion.csv`` の ``error`` 列を読む。すべて失敗したときは終了ステータス 1 になる（:ref:`page-howto-dispersion-run`）。
   * - CFD のツールがない、ケースが収束しない、メモリが足りない
     - :ref:`IY-O009`、``IY-F001``、``IY-F002``、``IY-F119``、``IY-F120``
     - ツールの入れ方は :ref:`page-howto-cfd-setup` にある。
   * - インストールが止まる
     - ``IY-I…``
     - :ref:`errors-install` と :ref:`page-howto-install`。

対処したあと、同じコマンドを実行し直して同じコードが出なければ、解消している。終了ステータスが 3 のままのときは、``INCOMPLETE RESULTS`` または ``RESULTS NOT USABLE`` の行を読み直す。
