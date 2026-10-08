.. _page-howto-external-table:
.. _sec-usage-external-table:

外部の係数表を使う
====================

風洞試験や他の解析で得た空力係数を、``aero.method = "table"`` で飛翔計算に使う手順を示す。
このページを終えると、手持ちの係数表で ``sim`` と ``dispersion`` を実行し、表の範囲を外れていないかを確認できる。

前提
------

* 係数表を CSV で持っている。列の名前と順序は、このソフトが書く ``aero_table.csv`` と同じである（:ref:`sec-aero-table-format`）。
* 表の基準量（基準面積、基準直径、全長）と格子（Mach 数、迎角）が分かっている。
* 機体の質量・重心・慣性とモータは、通常どおり ``[rocket]`` と ``[motor]`` で与える。係数表が置き換えるのは空力係数だけである。

手順
------

1. **CSV を用意する。** ``out/aero_table.csv`` を手本にして、``ext_table.csv`` という名前で、見出し行と同じ列を、Mach 数を外側、迎角を内側にして全格子点を 1 行ずつ並べる。
   一般表（迎角と横滑り角の 2 次元）では、横滑り角を最も内側にする。列の定義は :ref:`sec-file-axisym` と :ref:`sec-file-general` にある。

   期待する結果： 見出し行が手本と一致し、全セルが有限の数値である。

2. **付随ファイルを用意する。** CSV と同じ名前で拡張子だけ ``.json`` にしたファイル（``ext_table.json``）を、同じディレクトリに置く。
   必須の項目は ``source_hash``\ （外部の表では任意の文字列。たとえば表の出所を表す名前）``ref_area`` ``ref_diameter`` ``length`` ``machs`` ``alphas_deg`` ``extrapolation`` である（:ref:`sec-file-sidecar`）。
   一般表では ``kind = "general"`` と ``betas_deg`` を加える。``machs`` と ``alphas_deg``\ （一般表では ``betas_deg`` も）は CSV の格子と一致させる。

   期待する結果： 付随ファイルが CSV の隣にあり、各軸の点数の積が CSV の行数と一致する。

3. **設定に 2 行書く。** 自分の設定ファイル（ここでは ``sample.toml``）の ``[aero]`` を次のようにする。

   .. code-block:: toml

      [aero]
      method = "table"
      table  = "ext_table.csv"

   ``table`` の相対パスは設定ファイルのあるディレクトリから解決される。
   STL からの形状抽出と空力モデルの計算は行われず、``sim`` と ``dispersion`` は表をそのまま使う。

4. **実行する。** 手順 3 で書き換えた設定ファイルで ``sim`` を実行する。

   .. code-block:: console

      $ ignisyeet sim sample.toml

   期待する結果： 先頭近くに ``Using external aero table ext_table.csv (13741 rows)`` の形の行が出て、続けて飛翔の結果が表示される。
   ここに示した行数は、``examples/out/aero_table.csv``\ （Mach 0 から 3、刻み 0.02、迎角 0 から 90 deg、刻み 1 deg）を、付随ファイルとともに ``ext_table.csv`` として外部の表に読ませたときの値である。
   このとき飛翔の結果は、同じ機体を ``method = "barrowman"`` で計算したときと一致する（apogee 4440.5 m、着地 E -655.6 m、N -7707.7 m）。

5. **軸対称の表を一般表として扱う（必要なときだけ）。** 軸対称の外部表を横滑りの連成を含む計算に使うときは、``aero.table_kind = "general"`` を加える。
   外部の表が最初から一般表なら、何も加えずにそのまま使われる。軸対称の表と一般表のどちらを使うかの判断は :ref:`page-theory-aero-tables` にある。

確認
------

* ``out/summary.json`` の ``aero_table_range`` を見る。``machs_in_range`` と ``angles_in_range`` がともに ``true`` なら、飛翔中の全引き出しが表の格子の中に収まっている。
  上の例では ``lookups`` が 54209 回、``max_abs_alpha_deg`` が 5.82 deg、``max_mach`` が 1.54 で、範囲外は 0 である。
* 表の範囲を外れると、``sim`` と ``dispersion`` は ``RESULTS NOT USABLE: [IY-O024] …`` の行を書き、終了ステータス 3 で終わる（:ref:`IY-O024`、:ref:`sec-exit-status`）。
  迎角の上限を 90 deg 以上にして表を作り直す。軸対称の表では、このとき範囲外の引き出しは起きない。
* 表のロール項（``cl_delta`` と ``clp``）がすべて 0 のときは、現在の STL から抽出したフィンの寸法で補われ、端末に注意書きが出る。
  ロール項を含む表は、そのまま使われる。
* 表の値の定義（座標系、モーメントの基準点、減衰の和）が :ref:`sec-aero-table-format-external` に従っているか確かめる。

うまくいかないとき
--------------------

.. list-table::
   :header-rows: 1
   :widths: 40 60

   * - 症状
     - 対処
   * - ``error[IY-C047]: aero.table is required when aero.method = "table"``
     - ``[aero]`` に ``table`` を書く（:ref:`IY-C047`）
   * - ``error[IY-A041]: cannot read aero table metadata …``
     - CSV と同じ名前の ``.json`` を同じディレクトリに置く（:ref:`IY-A041`）
   * - ``error[IY-A042]: bad aero table metadata …``
     - 付随ファイルの JSON の書式と項目の型を直す（:ref:`IY-A042`）
   * - ``error[IY-A043]: cannot read aero table …``
     - ``aero.table`` のパスとファイルの読み取り権限を確かめる（:ref:`IY-A043`）
   * - ``error[IY-A044]: aero table …: unexpected CSV header …``
     - 見出し行の列名と順序を手本と一致させる（:ref:`IY-A044`）
   * - ``error[IY-A045]``、``error[IY-A046]``、``error[IY-A047]``
     - 行の値が数値でない、値の数が見出しと違う、NaN か無限大がある。メッセージの行番号の行を直す（:ref:`IY-A045`、:ref:`IY-A046`、:ref:`IY-A047`）
   * - ``error[IY-A048]``、``error[IY-A049]``、``error[IY-A050]``
     - 行の座標が付随ファイルの格子と一致しない、または行数が格子の点数と合わない。行の並びは Mach 数が最も外側である（:ref:`IY-A048`、:ref:`IY-A049`、:ref:`IY-A050`）
   * - 終了ステータス 3 と ``RESULTS NOT USABLE: [IY-O024]``
     - 表の迎角の上限を 90 deg 以上にして作り直す（:ref:`IY-O024`）

関連
------

* 列と付随ファイルの項目の仕様：:ref:`page-reference-aero-table-format`
* 表の考え方、内挿と外挿、格子の刻み：:ref:`page-theory-aero-tables`
* 手法の選び方：:ref:`page-theory-aero-choose`
* 設定キー：:ref:`cfg-aero-method`、:ref:`cfg-aero-table`、:ref:`cfg-aero-table_kind`
* CFD で自分の係数表を作る：:ref:`page-howto-cfd-table`
