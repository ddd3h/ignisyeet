.. _page-howto-warnings-errors:

エラーとワーニングを読む
==========================

IgnisYeet が出すエラーやワーニングの行の読み方と、原因と対処が書かれたページの探し方を示す。
症状から調べるための対処表も付ける。各コードの意味、主な原因、対処は :ref:`errors` の一覧にあり、ここには重ねて書かない。

目的
------

* 画面に出た行から、エラー、ワーニング、結果の使用可否を見分ける。
* 行頭のコード（``IY-…``）から、対処が書かれたページを開く。

前提
------

* ``ignisyeet`` を実行して、``error[IY-…]:`` や ``warning[IY-…]:`` などの行が出ている。
* 終了ステータスを見られる（``echo $?``）。

手順
------

1. 終了ステータスを確かめる。コマンドの直後に ``echo $?`` を実行する。

   .. code-block:: sh

      ignisyeet sim rocket.toml
      echo $?

   期待する結果：``0``、``1``、``2``、``3`` のいずれかが出る。値ごとに見る場所が決まる（:numref:`fig-exit-status-flow`）。

   .. _fig-exit-status-flow:

   .. figure:: /_generated/tikz/exit_status_flow.*
      :width: 100%
      :alt: 終了ステータスから見る場所への流れ図

      終了ステータスから、読む行を決める流れ。左から、成功（0）、エラー（1）、使い方の誤り（2）、結果が不完全または使えない（3）である。見るのは、値ごとに読む行が違うことで、1 と 3 は行頭のコードを一覧で引く。

   終了ステータスの定義は :ref:`sec-exit-status` にある。

2. 行の種類を見分ける。コードは ``IY-`` に領域の英字 1 文字と 3 桁の番号が続く形（例：``IY-C026``）である。
   番号が 001 から 499 ならエラー、500 から 999 ならワーニングである。

   .. list-table::
      :header-rows: 1
      :widths: 34 26 40

      * - 行
        - 終了ステータス
        - 意味
      * - ``error[IY-C026]: invalid config …``
        - 1
        - 処理が止まった。原因があれば、空行のあとに ``Caused by:`` と原因の行が続く。コードは先頭の行に 1 回だけ出る。標準エラー出力に書く。
      * - ``warning[IY-O514]: …``
        - 0（結果を使えないものを除く）
        - 処理は続いた。結果の読み方に注意が要る。端末では黄色の ``⚠ [IY-O514] …`` で出る。``-q`` では標準エラー出力に出る。
      * - ``error: …``\ （コードが先頭にない）
        - 2
        - コマンドラインの使い方の誤り。``--help`` を見る。``--set`` の形の誤りでは、コードが文中に出る（``[IY-C002]``）。
      * - ``ignisyeet: INCOMPLETE RESULTS: [IY-O027] …``
        - 3
        - 出力は書いたが、結果が不完全。着地しなかった、または失敗したケースがある。
      * - ``ignisyeet: RESULTS NOT USABLE: [IY-O020] …``
        - 3
        - 出力は書いたが、結果が使えない。静的に不安定、静的な安定を判定できない、係数表の範囲外の引き出しがある。結果を判断に使わない。

   警告は ``summary.json``、``dispersion_meta.json``、``dispersion_summary.json``、``geometry.json`` の ``warnings`` にも、``[IY-O514] …`` のようにコードを角括弧のまま先頭に付けて残る。
   あとから結果のファイルだけを見るときは、この配列を読む。

   期待する結果：エラーなら ``error[`` の行、ワーニングなら ``warning[`` の行、終了ステータス 3 なら ``ignisyeet:`` で始まる行から、コードが見つかる。

   実例を示す。サンプルの ``sim.dt`` を ``1e-7`` にすると、次のエラーで止まる（終了ステータス 1）。

   .. code-block:: text

      error[IY-C026]: invalid config sample.toml with the --set overrides

      Caused by:
          sim.dt = 0.0000001 s is too small (>= 1e-5 s): the run would take days; 0.002 s is typical

   ``Caused by:`` の次の行が、直すべき項目と値の範囲を示す。

3. コードを一覧で引く。

   #. 行からコードの部分（``IY-C026``）を写す。``error[`` と ``]:`` は要らない。
   #. 英字から、領域のページを開く。

      .. list-table::
         :header-rows: 1
         :widths: 10 40 50

         * - 英字
           - 領域
           - ページ
         * - ``C``
           - 設定（設定ファイル、``--set``、``[resources]``、``[aero.cfd]``）
           - :ref:`errors-config`
         * - ``G``
           - 形状（STL の読み込みと抽出）
           - :ref:`errors-geometry`
         * - ``A``
           - 空力（係数表、大気、パネル法）
           - :ref:`errors-aero`
         * - ``S``
           - 飛行計算と分散計算（モータ、風の表、回収系、積分）
           - :ref:`errors-sim`
         * - ``O``
           - コマンド、出力、結果の使用可否
           - :ref:`errors-cli`
         * - ``F``
           - CFD（SU2 とメッシャー）
           - :ref:`errors-cfd`
         * - ``I``
           - インストーラ
           - :ref:`errors-install`

   #. ページ内でコードの見出しを探す（ブラウザのページ内検索か、文書の検索欄に ``IY-C026`` を入れる）。文書の中からは ``:ref:`IY-C026``` で参照できる。
   #. ``意味``、``主な原因``、``対処`` を読む。

   期待する結果：コードの見出しの下に、メッセージの形、意味、主な原因、対処が出る。

4. 症状から調べる。コードが複数出ているときや、コードが分からないときは、次の表で近い症状を探す。

.. _tbl-warnings-symptoms:

症状別の対処表
----------------

.. list-table::
   :header-rows: 1
   :widths: 24 28 48

   * - 症状
     - コード
     - 確かめる設定と、読むページ
   * - 設定ファイルが読めない、値が範囲外
     - :ref:`IY-C060`、:ref:`IY-C061`、:ref:`IY-C005`、:ref:`IY-C009` から :ref:`IY-C059`
     - メッセージが名指しする項目を直す。``--set`` の綴りの誤りは、未知のキーとして原因の行に出る。値の範囲の一覧は :ref:`sec-config-checks` にある。
   * - STL が読めない、表面が閉じていない
     - :ref:`IY-G001` から :ref:`IY-G011`
     - STL を修復するか、ソリッドとして書き出し直す（:ref:`page-howto-own-rocket`）。
   * - 形状の抽出結果が疑わしい（比、向き、フィンの位置）
     - :ref:`IY-G501` から :ref:`IY-G512`
     - ``rocket.stl_scale``、``rocket.nose_direction``、``rocket.fin_override`` を確かめる（:ref:`sec-geometry-warnings`）。
   * - 全長が範囲外、モータが機体に収まらない
     - :ref:`IY-O005` から :ref:`IY-O008`、:ref:`IY-O502`、:ref:`IY-O503`
     - ``rocket.stl_scale``、``rocket.cg_dry``、``motor.aft_x`` を確かめる。
   * - 推力曲線のファイルが読めない、疑わしい
     - :ref:`IY-S001` から :ref:`IY-S015`、:ref:`IY-S500` から :ref:`IY-S502`、:ref:`IY-O013`
     - ``.eng`` の行を直す（:ref:`page-howto-motor`）。
   * - レールを離れない
     - :ref:`IY-S080`
     - 推力、質量、``launch.rail_length`` を確かめる（:ref:`page-howto-launcher-parachute`）。
   * - ランチャ離脱速度が低い、風による迎角が大きい
     - :ref:`IY-O515`、:ref:`IY-O516`
     - ``launch.rail_length`` を長くする（:ref:`page-howto-launcher-parachute`）。
   * - 静的に不安定、または判定できない
     - :ref:`IY-O020`、:ref:`IY-O021`、:ref:`IY-O025`、:ref:`IY-O026`、:ref:`IY-O505` から :ref:`IY-O508`
     - 終了ステータス 3。``rocket.cg_dry`` とフィン、モータの取り付け位置を見直す。結果は使わない（:ref:`page-howto-read-results`）。
   * - 着地しない
     - :ref:`IY-O027`、:ref:`IY-O028`、:ref:`IY-O029`、:ref:`IY-O504`、:ref:`IY-O530`
     - 終了ステータス 3。降下方式、``recovery.cd_s``、``sim.max_time`` を確かめる（:ref:`page-howto-launcher-parachute`）。
   * - 係数表の範囲外の係数を使った
     - :ref:`IY-O022`、:ref:`IY-O023`、:ref:`IY-O024`
     - 終了ステータス 3。``aero.alpha_max_deg``、``aero.mach_max`` を大きくして表を作り直す。
   * - 風の格子のケースが失敗した
     - :ref:`IY-O011`、:ref:`IY-O531` から :ref:`IY-O533`
     - ``dispersion.csv`` の ``error`` 列を読む。すべて失敗したときは終了ステータス 1 になる（:ref:`page-howto-dispersion-run`）。
   * - 弾道落下が横向きに落ちた
     - :ref:`IY-O509`、:ref:`IY-O536`
     - 終了ステータスは変わらない。着地点は検証していない大迎角の空力で決まる（:ref:`page-theory-limitations`）。
   * - パラシュートが高速で開く、着地速度が大きい、開傘が遅れた
     - :ref:`IY-O511`、:ref:`IY-O517` から :ref:`IY-O520`
     - ``recovery.delay``、``recovery.cd_s``、``recovery.main_altitude`` を確かめる（:ref:`page-howto-launcher-parachute`）。
   * - 風の表が読めない、表が飛翔の高度に足りない
     - :ref:`IY-S030` から :ref:`IY-S037`、:ref:`IY-S050` から :ref:`IY-S053`、:ref:`IY-O512`、:ref:`IY-O513`、:ref:`IY-O521`
     - 表の行と高度の範囲を直す（:ref:`page-howto-environment`）。
   * - 頂点が高く、地上風のモデルが外挿になる
     - :ref:`IY-O514`
     - 高度 1 km を超える飛翔では、``wind.model = "profile"`` を使う（:ref:`page-howto-environment`）。
   * - 出力先を別の実行が使っている
     - :ref:`IY-O031`、:ref:`IY-O032`
     - ``--out`` で出力先を分ける（:ref:`sec-output-lock`）。
   * - ``--aero-table`` が受け付けられない
     - :ref:`IY-O002`、:ref:`IY-O003`、:ref:`IY-O004`
     - 表を同じ設定で作り直す（:ref:`page-howto-sweeps`）。
   * - モンテカルロの統計が粗い、失敗したサンプルがある
     - :ref:`IY-O537`、:ref:`IY-O538`、:ref:`IY-O029`
     - ``dispersion.monte_carlo.samples`` を増やす（:ref:`page-howto-dispersion-run`）。
   * - CFD のツールがない、ケースが収束しない、メモリが足りない
     - :ref:`IY-O009`、:ref:`IY-F001`、:ref:`IY-F002`、:ref:`IY-F119`、:ref:`IY-F120`
     - ツールの入れ方は :ref:`page-howto-cfd-setup`、ケースの扱いは :ref:`page-howto-cfd-table` にある。
   * - インストールが止まる
     - ``IY-I…``
     - :ref:`errors-install` と :ref:`page-howto-install`。

確認
------

* 行のコードを引き、``対処`` を実施したあと、同じコマンドを実行し直して、同じコードが出ない。
* 終了ステータスが 0 になる。3 のままのときは、``INCOMPLETE RESULTS`` または ``RESULTS NOT USABLE`` の行を読み直す。
* ``warnings`` に残っているコードの意味を、すべて理解している。

うまくいかないとき
--------------------

コードのページが見つからない
   行頭のコードの綴りを確かめる。``error[IY-C005]`` のように、先頭の行のコードを使う。文中に別のコードが入るメッセージ（風の格子のケースの失敗にその原因のコードが入るものなど）では、先頭のコードが外側の状況を表し、文中のコードが原因を表す。

コードが出ずに ``error:`` だけが出る
   コマンドラインの使い方の誤りである。``ignisyeet --help`` と ``ignisyeet <サブコマンド> --help`` を見る。

警告が多くて見落とす
   ``summary.json`` の ``warnings`` に、計算中の警告がすべて残る。コードを列挙して、症状別の対処表で引く。

関連
------

* :ref:`errors`：コードの一覧と、表示の形
* :ref:`sec-exit-status`：終了ステータスの定義
* :ref:`page-howto-read-results`：結果パネルと ``summary.json`` の読み方
* :ref:`sec-config-checks`：設定の値域の検査
