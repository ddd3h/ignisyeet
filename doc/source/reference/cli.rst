.. _page-reference-cli:

コマンドライン
================

コマンド ``ignisyeet`` の引数と、補助の ``make`` と ``python/plot.py`` を引く。
サブコマンドの使い方は :ref:`page-howto` の各ページで示す。

.. _sec-cli-subcommands:

サブコマンド
------------

``ignisyeet [-q] [--no-progress] <サブコマンド> <config> [オプション]`` の形で呼ぶ。``<config>`` は設定ファイルのパスである。

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - コマンド
     - 内容と書き出すファイル
   * - ``geom <config>``
     - STL から形状を抽出する。``geometry.json`` と ``profile.csv`` を書く。
   * - ``aero <config> [--force]``
     - 空力係数表 ``aero_table.csv`` を作る。入力と設定が前回と同じなら保存済みの表を使う。``--force`` を付けると、保存済みの表が設定と一致していても再計算する（``aero.method = "table"`` では効果がない）。
   * - ``sim <config> [--descent ballistic|parachute]``
     - 設定した風で 1 回飛翔させる。``trajectory.csv`` と ``summary.json`` を書く。``--descent`` は ``sim.descent`` を上書きする。係数表がないか入力が変わったときは、先に ``aero`` を実行する。
   * - ``dispersion <config>``
     - ``dispersion.mode`` に従って落下分散を並列に計算する。``"wind_grid"`` は ``dispersion.csv`` と ``dispersion_meta.json`` を、``"monte_carlo"`` は ``dispersion_mc.csv`` と ``dispersion_summary.json`` を書く。係数表がないか入力が変わったときは、先に ``aero`` を実行する。
   * - ``cfd-check <config> [--mesh]``
     - CFD に必要なツール（SU2、MPI、gmsh。``surface = "stl"`` では manifold3d も）の有無と版、ケース数を表示する。``--mesh`` は壁面のメッシュも作って品質を表示する。``surface = "stl"`` では形状の忠実度、``surface = "step"`` では CAD のモデルの読み込みと位置合わせも表示する。必要なツールが足りないときは終了ステータス 1 で終わる。
   * - ``sample-stl <path>``
     - 付属のサンプル機体の STL（mm 単位、ノーズ +z 向き）を ``<path>`` に書く。

各コマンドは、実行の開始時に同じ種類の前回の結果を削除する（:ref:`page-reference-output-files`）。
終了ステータスは :ref:`sec-exit-status` に定義する。

共通のオプション
~~~~~~~~~~~~~~~~

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - オプション
     - 内容
   * - ``-q`` ``--quiet`` ``--no-progress``
     - 出力の量と進捗表示の指定。:ref:`sec-ui-modes` に定義する。
   * - ``--set KEY=VALUE``
     - ``geom`` ``aero`` ``sim`` ``dispersion``。設定のキーを上書きする（:ref:`sec-cli-overrides`）。
   * - ``--out DIR``
     - ``geom`` ``aero`` ``sim`` ``dispersion``。出力先を ``output.dir`` の代わりに ``DIR`` にする。
   * - ``--aero-table PATH``
     - ``aero`` ``sim`` ``dispersion``。別の出力先で作った係数表を使う（:ref:`sec-cli-overrides`）。
   * - ``--help`` ``--version``
     - 使い方、版を表示する。

.. _sec-cli-overrides:

設定の上書き
------------

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

.. code-block:: sh

   ignisyeet aero examples/sample.toml --set aero.method=panel --out runs/aero_panel
   ignisyeet sim  examples/sample.toml --set aero.method=panel --aero-table runs/aero_panel \
       --set wind.speed=7 --set wind.direction_deg=90 --out runs/sim_panel_7ms

.. _sec-ui-modes:

出力モードと環境変数
--------------------

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
     - エラー、警告、書き出したファイルのパスだけを表示する。警告は標準エラー出力に ``warning[IY-…]:`` の行で書く（:ref:`page-reference-error-codes`）。
   * - ``--no-progress``
     - 進捗バーとスピナーを出さない。枠付きの要約は表示する。
   * - ``NO_COLOR`` が空でない値で設定されている
     - 色を使わない。枠や記号は残る。
   * - ``TERM=dumb``
     - 色と Unicode の記号を使わない。
   * - ロケールが UTF-8 でない（``LANG`` ``LC_ALL`` ``LC_CTYPE`` が UTF-8 を指さない、または ``TERM=linux``）
     - 枠線・記号・バーを ASCII 文字（``+ - |``、``ok`` ``x`` ``!``、``#>-``）に置き換える。

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
     - ``aero`` ``sim`` ``dispersion`` ``plot`` を順に実行する。``sim`` か ``dispersion`` が終了ステータス 3 で終わっても作図まで進み、最後に ``make: results are INCOMPLETE or NOT USABLE (exit status 3): see the lines above`` を出して失敗する（終了ステータスは :ref:`sec-exit-status`）。ほかの失敗ではその場で止まる。
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

.. _sec-cli-plot:

作図コマンド
------------

``python/plot.py``\ （パッケージ名 ``ignisyeet-plot``）は ``plot.py <図> <出力ディレクトリ> [--compare DIR]`` の形で呼ぶ。

.. list-table::
   :header-rows: 1
   :widths: 24 76

   * - 図
     - 内容
   * - ``all``
     - 出力ディレクトリにある結果から描けるすべての図を ``<出力ディレクトリ>/plots/`` に書く。
   * - ``geometry``
     - 形状の図。``profile.csv`` を使う。
   * - ``aero``
     - 空力係数の図。``--compare DIR`` を付けると、二つの係数表の :math:`C_{N\alpha}` :math:`x_{cp}` :math:`C_{D0}` を Mach 数に対して重ねた ``aero_compare.png`` も書く。
   * - ``panel``
     - パネル法の表面圧力係数の図 ``panel.png``。``panel_cp.csv`` を使う。
   * - ``trajectory``
     - 飛翔経路の図。``trajectory.csv`` と ``summary.json`` を使う。
   * - ``dispersion``
     - 落下分散の図。``dispersion.csv`` があれば ``dispersion.png``、``dispersion_mc.csv`` と ``dispersion_summary.json`` があれば ``dispersion_mc.png`` を書く。

出力ディレクトリがないとき、描けるものが一つもないときは、エラーを表示して終了ステータス 1 で終わる。
図の内容は :ref:`page-howto-plots-kml` に示す。
