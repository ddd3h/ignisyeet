.. _page-reference-exit-codes:

終了ステータス
================

``ignisyeet`` の終了ステータスと、終了ステータス 3 の意味を定義する。対処の手順は :ref:`page-howto-warnings-errors` にある。

.. _sec-exit-status:

終了ステータスの定義
--------------------

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
----------

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

``make all`` での扱いは :ref:`sec-cli-make` にある。
