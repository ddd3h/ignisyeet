.. _page-reference-installer:

インストーラ
==============

``install.sh`` のオプションと環境変数を引く。手順は :ref:`page-howto-install` にある。

.. _sec-installer-options:

オプション
----------

質問に答える代わりに、オプションで指定できる。``curl ... | bash`` の場合は、オプションを ``bash -s --`` の後ろに付ける。

.. code-block:: sh

   curl -fsSL https://raw.githubusercontent.com/ddd3h/ignisyeet/main/install.sh | bash -s -- --developer --cfd -y

.. list-table::
   :header-rows: 1
   :widths: 30 70

   * - オプション
     - 内容
   * - ``--user`` ``--developer``
     - 役割を尋ねずに指定する。``--dev`` は ``--developer`` の別名である。
   * - ``--cfd`` ``--no-cfd``
     - CFD ツール（SU2、gmsh、Open MPI と manifold3d 3.5.4）を入れる、入れない。
   * - ``--su2-binary``
     - ``--cfd`` と併せて、SU2 のソース構築を省いて移植できる conda-forge の SU2 を使う。指定しないときは、Linux で SU2 8.3.0 をその CPU 向けに構築する。
   * - ``--version vX.Y.Z``
     - ``--version=vX.Y.Z`` の形でもよい。入れるリリース（既定は最新）。developer ではチェックアウトするタグ。形式は ``vX.Y.Z`` に限り、``v`` のない値や他の形式はエラーになる。
   * - ``--prefix DIR``
     - ``--prefix=DIR`` の形でもよい。インストール先（既定は ``~/.local``）。相対パスと ``~`` で始まるパスは、起動時に絶対パスへ直す。
   * - ``--dir DIR``
     - ``--dir=DIR`` の形でもよい。developer の clone 先（既定は実行した clone、なければ ``~/ignisyeet``）。``--prefix`` と同様に絶対パスへ直す。
   * - ``--no-test``
     - developer で、インストール時の ``cargo test --release`` を実行しない。
   * - ``--add-path``
     - ``PATH`` の行をシェルの設定ファイルに、尋ねずに追記する。
   * - ``--force``
     - 同じバージョンが入っていても入れ直す。
   * - ``-y`` ``--yes``
     - 質問せず既定値を使う（user、CFD なし、シェル設定ファイルは書き換えない）。
   * - ``--no-color``
     - 色を使わない出力にする（``NO_COLOR`` を設定しても同じになる）。
   * - ``--dry-run``
     - 計画だけを表示して終了する。
   * - ``--uninstall``
     - バイナリ、リンク、リリースデータを削除する。作図環境と CFD 環境は確認してから削除する（``--purge`` で両方を確認なしに削除）。developer の clone は明示的に確認したときだけ削除する。
   * - ``--purge``
     - ``--uninstall`` と併せて、作図環境、CFD 環境、SU2 のソース構築を確認なしに削除する。
   * - ``-h`` ``--help``
     - オプションの一覧を表示して終了する。

.. _sec-installer-env:

環境変数
--------

各オプションには環境変数が対応する。

.. list-table::
   :header-rows: 1
   :widths: 40 60

   * - 環境変数
     - 対応するオプションなど
   * - ``IGNISYEET_ROLE``
     - ``--user`` ``--developer``\ （値は ``user`` または ``developer``）
   * - ``IGNISYEET_VERSION``
     - ``--version``
   * - ``IGNISYEET_PREFIX``
     - ``--prefix``
   * - ``IGNISYEET_DIR``
     - ``--dir``
   * - ``IGNISYEET_CFD``
     - ``--cfd``\ （``1``）、``--no-cfd``\ （``0``）
   * - ``IGNISYEET_SU2_BINARY``
     - ``--su2-binary``\ （``1``）
   * - ``IGNISYEET_SU2_JOBS``
     - SU2 のソース構築の並列数。既定は全コア数で、利用可能なメモリ 2 GB につき 1 並列を上限とする。
   * - ``IGNISYEET_YES`` ``IGNISYEET_DRY_RUN`` ``IGNISYEET_FORCE`` ``IGNISYEET_ADD_PATH``
     - ``-y``、``--dry-run``、``--force``、``--add-path``\ （``1`` で有効）
   * - ``IGNISYEET_NO_COLOR``
     - 色を使わない（``NO_COLOR`` を設定しても同じになる）。
   * - ``IGNISYEET_REPO``
     - GitHub のリポジトリ名（既定は ``ddd3h/ignisyeet``）。次の 2 つの既定値はここから決まる。
   * - ``IGNISYEET_DOWNLOAD_BASE`` ``IGNISYEET_REPO_URL``
     - リリースの取得元の URL、developer が clone する git の URL。

``NO_COLOR`` を設定するか、端末でない場所へ出力すると、装飾のない行単位の出力になる。
端末でないとき（``-y`` を付けた場合やパイプ越しで入力がない場合を含む）は質問せず、既定値（user、CFD なし、``PATH`` は書き換えない）を使う。
リリースの配布物は ``ignisyeet-<バージョン>-<ターゲット>.tar.gz`` と ``.sha256`` で、インストーラはチェックサムを検証してから展開する。

.. _sec-installer-su2:

SU2 の選択とソース構築
----------------------

``--cfd`` は、Linux では SU2 8.3.0 をその CPU 向けにソースから構築する。``--su2-binary`` を指定すると、構築せずに conda-forge の SU2 を使う。
構築には CFD 環境の gcc/g++ を使い、``PREFIX/share/ignisyeet-su2-native`` に約 70 MB が残る。ここにある ``build-info.txt`` に、コンパイラの版と CPU のモデルが記録される。
構築した SU2 は、構築した CPU と同じ種類の CPU でだけ動く。

CFD の計算で使う SU2 は、次の順で最初に見つかったものである。

#. ``[aero.cfd]`` の ``su2``\ （既定の ``SU2_CFD`` 以外を指定したとき）
#. 環境変数 ``IGNISYEET_SU2_CFD``
#. 構築した ``share/ignisyeet-su2-native/bin/SU2_CFD``\ （インストール先、``IGNISYEET_PREFIX``、``~/.local`` の順に探す）
#. ``[aero.cfd]`` の ``prefix`` と ``PATH`` にある ``SU2_CFD``\ （conda-forge のバイナリ）

``mpirun``、gmsh、ライブラリは、どの場合も ``[aero.cfd]`` の ``prefix`` の CFD 環境のものを使う。
``--prefix`` で既定以外のインストール先にしたときは、実行時に ``IGNISYEET_PREFIX`` を同じ場所に設定しないと、構築した SU2 が見つからない。

``--su2-binary`` を指定すると、すでにある ``share/ignisyeet-su2-native`` は削除される。
``--uninstall`` は ``share/ignisyeet-su2-native`` を削除するか尋ね、``--purge`` は尋ねずに削除する。
構築の並列数は ``IGNISYEET_SU2_JOBS`` で決める（:ref:`sec-installer-env`）。

.. _sec-installer-uninstall:

アンインストール
----------------

``bash install.sh --uninstall`` を実行する。
バイナリ、``ignisyeet-plot`` のリンク、リリースデータを削除するか尋ねたあと、作図用の環境と CFD の conda 環境（作成していた場合）を削除するかをそれぞれ尋ねる（:numref:`fig-screen-install-uninstall`）。
developer の clone は、明示的に確認したときだけ削除する。
``--purge`` を付けると、作図環境と CFD 環境も尋ねずに削除する。
``install.log`` と、シェルの設定ファイルに追記した ``PATH`` の行は残る。

.. _fig-screen-install-uninstall:

.. figure:: /_generated/screens/screen_install_uninstall.png
   :width: 100%
   :alt: install.sh --uninstall の実行画面

   ``install.sh --uninstall`` の画面。バイナリとリリースデータ、作図環境の削除に ``y`` と答えたところである。

.. _sec-installer-update:

更新と再インストール
--------------------

インストール用のコマンドをもう一度実行すると更新になる。入っていないものだけを入れる。
user では最新のリリースが取得され、``current`` のリンクが新しい版に切り替わり、古い版のデータは ``~/.local/share/ignisyeet/<バージョン>`` に残る。
特定のバージョンは ``--version vX.Y.Z`` で指定する。
同じバージョンがすでに入っているときは ``Reinstall it?`` と尋ねる。尋ねない場面（``-y`` など）で入れ直すには ``--force`` を付ける。
インストールの全体の記録は ``~/.local/share/ignisyeet/install.log`` にある。

.. _sec-installer-symptoms:

表示とその意味
--------------

.. list-table::
   :header-rows: 1
   :widths: 32 68

   * - 表示
     - 意味
   * - ``could not determine the latest release``
     - 最新のリリースが分からない。リリースが公開されていないか、ネットワークに接続できていない。``--developer`` か ``--version vX.Y.Z`` で入れられる。
   * - ``ignisyeet: command not found``
     - ``~/.local/bin`` が ``PATH`` に入っていない。
   * - プロキシの内側でのダウンロードの失敗
     - ``curl`` は ``https_proxy`` と ``http_proxy`` を読む。
   * - 失敗した手順の名前とログの末尾
     - 途中で失敗した。全体の記録は ``install.log`` にある。インストーラは再実行できる。
   * - CFD ツールでの SU2 の異常終了
     - AVX-512 のない CPU では SU2 8.5.0 が異常終了する。インストーラが作る環境は SU2 8.3.0 に固定している。別の版が入った環境は、画面に出る再作成のコマンドで作り直せる。

エラーコードは :ref:`errors-install` にある。
