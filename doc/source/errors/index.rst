.. _errors:

.. _page-reference-error-codes:

エラーとワーニングの一覧
========================

IgnisYeet が出すエラーとワーニングには、安定したコードが付く。
コードは ``IY-`` に領域の英字 1 文字と 3 桁の番号が続く形（例：``IY-C012``）で、メッセージの文面を直しても変わらない。
各コードの意味、主な原因、対処は、領域ごとの一覧にある。

番号が 001 から 499 ならエラー、500 から 999 ならワーニングである。
エラーは処理を止める。ワーニングは処理を続け、結果の読み方に注意が要ることを知らせる。
ただし、結果を使えない状況のワーニング（静的な不安定など）は、出力を書いた後に終了ステータス 3 で終わる。

エラーは ``error[IY-Xnnn]: …``、ワーニングは ``warning[IY-Xnnn]: …`` の形で、メッセージの先頭にコードを付けて表示する。
同じ状況は呼び出し箇所によらず同じコードで表示する。表示の形は出力モードで変わる（:ref:`sec-ui-modes`）。

.. list-table::
   :header-rows: 1
   :widths: 22 78

   * - 出力
     - 形
   * - プレーンモード（標準出力がパイプやファイルのとき）
     - ワーニングは標準出力に ``warning[IY-O515]: …`` の行で書く。
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
コマンドラインの引数の誤り（``--set`` の形など）は ``error: invalid value '…' for '--set <KEY=VALUE>': [IY-C002] …`` の形で、コードは文中に出る。コードが先頭になく ``error: …`` だけの行は、終了ステータス 2 の使い方の誤りである。``ignisyeet --help`` と ``ignisyeet <サブコマンド> --help`` を見る。

機械出力では、コードを ``[IY-Xnnn] …`` の形のまま文字列の先頭に書く。
次のものがそうである。

* ``summary.json``、``dispersion_meta.json``、``dispersion_summary.json``、``geometry.json`` の ``warnings`` の配列（:ref:`sec-file-summary`）。
* 終了ステータス 3 の行 ``ignisyeet: RESULTS NOT USABLE: [IY-O020] …`` と ``ignisyeet: INCOMPLETE RESULTS: [IY-O0nn] …``\ （:ref:`sec-exit-status`）。

ワーニングの数値のうち、しきい値と比べるものは、しきい値を破った側にあると読める桁数で示す（静安定余裕 1.4977 cal は ``1.498``、開傘の速さ 50.2 m/s は ``50.2``）。
ワーニングは ``summary.json`` の ``warnings`` に、落下分散では ``dispersion_meta.json`` と ``dispersion_summary.json`` の ``warnings`` にも、同じ文を残す。設定の読み込み時のワーニング（``profile`` で使わない ``wind.speed`` と ``wind.direction_deg``、2 回以上指定した ``--set`` のキー）は、配列の先頭に入る。後から結果のファイルだけを見るときは、この配列を読む。

実例を示す。サンプルの ``sim.dt`` を ``1e-7`` にすると、次のエラーで止まる（終了ステータス 1）。

.. code-block:: text

   error[IY-C026]: invalid config sample.toml with the --set overrides

   Caused by:
       sim.dt = 0.0000001 s is too small (>= 1e-5 s): the run would take days; 0.002 s is typical

``Caused by:`` の次の行が、直すべき項目と値の範囲を示す。

コードの探し方
--------------

行からコードの部分（``IY-C026``）を写し、英字から領域の一覧を開く。``error[`` と ``]:`` は要らない。致命的エラーに ``Caused by:`` がある場合は、先頭の行のコードを探す。
一覧の見出しはコードそのものなので、ブラウザのページ内検索か、文書の検索欄にコードを入れて探せる。各コードのアンカーは ``IY-C012`` の形である。ソースやログを ``grep -r "IY-C012"`` で探すと、そのコードを出す箇所もわかる。
各コードの項目には、メッセージの形、意味、主な原因、対処が並ぶ。

.. list-table::
   :header-rows: 1
   :widths: 10 50 40

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
     - 飛行計算と分散計算（モータ、風の表、回収系、積分）
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

領域ごとの一覧を次に示す。

.. toctree::
   :maxdepth: 1

   config
   geometry
   aero
   sim
   cli
   cfd
   install
