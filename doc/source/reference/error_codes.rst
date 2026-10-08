.. _errors:

.. _page-reference-error-codes:

エラーとワーニング
==================

IgnisYeet が出すエラーとワーニングには、安定したコードが付く。
コードは ``IY-`` に領域の英字 1 文字と 3 桁の番号が続く形（例：``IY-C012``）で、メッセージの文面を直しても変わらない。
コードをこの文書で検索すると、意味、主な原因、対処が見つかる。

コードの表示
------------

エラーは ``error[IY-Xnnn]: …``、ワーニングは ``warning[IY-Xnnn]: …`` の形で、メッセージの先頭にコードを付けて表示する。
同じ状況は呼び出し箇所によらず同じコードで表示する。

.. list-table::
   :header-rows: 1
   :widths: 22 78

   * - 出力
     - 形
   * - プレーンモード（標準出力がパイプやファイルのとき）
     - ワーニングは標準出力に ``warning[IY-O515]: rail exit speed … m/s is below 15 m/s: …`` の行で書く。
   * - クワイエットモード（``-q``）
     - ワーニングを標準エラー出力に、プレーンモードと同じ ``warning[IY-…]: …`` の行で書く。エラーも表示する。
   * - リッチモード（標準出力が端末のとき）
     - ワーニングは黄色の ``⚠ [IY-O515] rail exit speed …`` の行で書く（``⚠`` が重大度を示すので、``warning`` の語は付けない）。
   * - 致命的エラー（コマンドが失敗して終了するとき）
     - 全モード共通で、標準エラー出力に ``error[IY-C062]: invalid config rocket.toml`` のように見出しの 1 行を書く。原因があれば、空行の後に ``Caused by:`` と原因の行が続く。コードは先頭の行に 1 回だけ出る。
   * - インストーラ（``install.sh``）
     - ``error[IY-I010]: …`` と ``warning[IY-I510]: …`` を標準エラー出力に書く。

CFD のケースごとの行は、コードの後ろに範囲を示す文が続く
（``error[IY-F118]: aero.cfd: case m0.500_a00.00 failed: …``、``warning[IY-F523]: aero.cfd: case m0.500_a00.00: …``）。
別のコードのエラーを文中に含むメッセージ（風の格子のケースの失敗にその原因のコードが入るもの、HXT と Delaunay の両方が失敗したメッシュのエラー）では、先頭のコードが外側の状況を表し、文中のコードが原因を表す。
コマンドラインの引数の誤り（``--set`` の形など）は、clap が組み立てる ``error: invalid value '…' for '--set <KEY=VALUE>': [IY-C002] …`` の形で、コードは文中に出る。

機械出力では、コードを ``[IY-Xnnn] …`` の形のまま文字列の先頭に書く。
次のものがそうである。

* ``summary.json``、``dispersion_meta.json``、``dispersion_summary.json``、``geometry.json`` の ``warnings`` の配列（:ref:`sec-file-summary`）。
* 終了ステータス 3 の行 ``ignisyeet: RESULTS NOT USABLE: [IY-O020] …`` と ``ignisyeet: INCOMPLETE RESULTS: [IY-O0nn] …``\ （:ref:`sec-exit-status`）。

領域
----

.. list-table::
   :header-rows: 1
   :widths: 10 30 60

   * - 英字
     - 領域
     - 一覧
   * - C
     - 設定（設定ファイル、``--set``、``[resources]``、``[aero.cfd]``）
     - :ref:`errors-config`
   * - G
     - 形状（STL の読み込みと抽出）
     - :ref:`errors-geometry`
   * - A
     - 空力（係数表、大気、パネル法）
     - :ref:`errors-aero`
   * - S
     - 飛行計算と分散計算
     - :ref:`errors-sim`
   * - O
     - コマンド、出力、結果の使用可否
     - :ref:`errors-cli`
   * - F
     - CFD（SU2 とメッシャー）
     - :ref:`errors-cfd`
   * - I
     - インストーラ
     - :ref:`errors-install`

番号と重大度
------------

番号は 001 から 499 がエラー、500 から 999 がワーニングである。
エラーは処理を止める。ワーニングは処理を続け、結果の読み方に注意が要ることを知らせる。
ただし、結果を使えない状況のワーニング（静的な不安定など）は、出力を書いた後に終了ステータス 3 で終わる。

終了ステータス
--------------

終了ステータスの定義は :ref:`sec-exit-status` にある。

コードの探し方
--------------

#. 画面や記録の行から、``IY-C012`` のようなコードの部分を写す。``error[`` と ``]:`` は要らない。
#. この文書の検索欄にコードを入れる。または、各一覧の見出しがコードそのものなので、ブラウザのページ内検索で一覧のページを探す。
#. 各コードのアンカーは ``IY-C012`` の形である。文書の中からは ``:ref:`IY-C012``` で参照できる。
#. ソースやログを ``grep -r "IY-C012"`` で探すと、そのコードを出す箇所がわかる。

致命的エラーに ``Caused by:`` がある場合は、先頭の行のコードを探す。

.. toctree::
   :maxdepth: 1

   ../errors/config
   ../errors/geometry
   ../errors/aero
   ../errors/sim
   ../errors/cli
   ../errors/cfd
   ../errors/install
