.. _page-howto-install:

インストール
============

この章は、IgnisYeet のインストールと更新、アンインストールを扱う。計算に使う ``ignisyeet`` コマンドと、作図に使う ``ignisyeet-plot`` コマンドを使えるようにするのが目的である。
CFD による空力計算を使うときだけ、CFD ツール（SU2、gmsh、Open MPI）も入れる。Barrowman 法とパネル法だけなら要らない。
中心はインストーラ ``install.sh`` を使う方法で、developer の入れ方、ソースからのビルド、``pip`` による作図環境も述べる。最後に、インストーラのオプションと環境変数を一覧にする。

前提
----

* 対応する OS：Linux（x86_64、aarch64）または macOS（Intel、Apple Silicon）。
* ``bash``、``curl``、``tar``\ （ほとんどの環境に最初から入っている）。
* インターネット接続（リリースの取得と、作図用ライブラリの導入に使う）。
* 管理者権限（``sudo``）は要らない。すべて自分のホームディレクトリの下（既定は ``~/.local``）に入る。

``git``、``cargo``、``uv``、``python3`` などは、user では必須ではない。
``python3`` が 3.10 未満または無いときは、インストーラが ``uv`` を使って用意することを提案する。

インストーラで入れる
--------------------

次の 1 行を端末に貼り付けて実行する。

.. code-block:: sh

   curl -fsSL https://raw.githubusercontent.com/ddd3h/ignisyeet/main/install.sh | bash

インターネット上のスクリプトをそのまま実行することに抵抗があるときは、いったん保存して中身を読んでから実行するとよい。

.. code-block:: sh

   curl -fsSLO https://raw.githubusercontent.com/ddd3h/ignisyeet/main/install.sh
   less install.sh
   bash install.sh

ロゴが出たあと、質問が始まる。``curl ... | bash`` で実行しても、質問は端末から読み取るのでそのまま答えられる。
質問は次の順に進む。

1. 役割を選ぶ（:numref:`fig-screen-install-role`）。``1`` が user（既定）、``2`` が developer で、Enter だけで user になる。
   user は、GitHub のリリースから自分のプラットフォーム用の実行ファイルを取得し、チェックサムを検証してから ``~/.local/bin/ignisyeet`` に置く。
   例題と作図スクリプトは ``~/.local/share/ignisyeet/<バージョン>`` に入り、``current`` というリンクが最新版を指す。
   作図用の環境と、それを使う ``ignisyeet-plot`` コマンドも用意する。developer は、ソースからビルドして開発する人向けである（次節）。

   .. _fig-screen-install-role:

   .. figure:: /_generated/screens/screen_install_role.png
      :width: 100%
      :alt: インストーラの起動画面と役割の質問

      インストーラの起動画面。``1``\ （user）を選んだところである。

2. CFD ツールを入れるか選ぶ（:numref:`fig-screen-install-cfd`）。
   インストーラは、すでに入っているツール（cargo、uv、python3、git、conda や micromamba、既存の ``ignisyeet-cfd`` 環境、SU2、gmsh など）を検出して ``Environment`` に一覧にする。
   行頭には、見つかったもの（緑のチェック）と見つからなかったもの（黄色の丸）が並ぶ。
   続けて CFD ツールを入れるかを尋ねる。既定は ``n``\ （入れない）で、すでに SU2 が見つかっているときは質問しない。質問を省くには ``--cfd`` または ``--no-cfd`` を指定する。

   .. _fig-screen-install-cfd:

   .. figure:: /_generated/screens/screen_install_cfd.png
      :width: 100%
      :alt: 環境の検出結果と CFD ツールの質問

      検出したツールの一覧と CFD ツールの質問。``n`` と答えたところである。

3. 計画を確認する（:numref:`fig-screen-install-plan`）。
   取得するファイル名、置き場所、作る環境が ``Plan`` として順に並び、``Proceed?`` と尋ねられる。この時点では何も変更していないので、内容が気に入らなければ ``n`` で中止できる。
   ``y`` または Enter で進む。

   .. _fig-screen-install-plan:

   .. figure:: /_generated/screens/screen_install_plan.png
      :width: 100%
      :alt: 実行計画の一覧と確認

      実行計画と確認。``y`` と答えて実行に進む。

4. インストールの進行を待つ（:numref:`fig-screen-install-progress`）。
   各手順にスピナー（回転する記号）が付き、ダウンロードでは進み具合を示すバーとサイズが出る。終わった手順には緑のチェックと所要時間が付く。
   作図用環境の作成は、初回はライブラリ（numpy、pandas、matplotlib）の取得のため数十秒かかることがある。
   詳しい出力はすべて ``~/.local/share/ignisyeet/install.log`` に記録される。

   .. _fig-screen-install-progress:

   .. figure:: /_generated/screens/screen_install_progress.png
      :width: 100%
      :alt: ダウンロード中の進行バー

      ダウンロード中の画面。進行バーと、取得済みのサイズ・全体のサイズが出る。

最後に ``IgnisYeet is ready`` の枠が出て、インストールされたバージョン、実行ファイル、データ、作図環境の場所と、次に実行するコマンドが書かれている（:numref:`fig-screen-install-done` は :ref:`page-tutorial` の章にある）。

.. _sec-install-path:

PATH を設定する
~~~~~~~~~~~~~~~

実行ファイルを置く ``~/.local/bin`` が ``PATH`` に入っていないときは、インストーラがシェルの設定ファイル（bash なら ``~/.bashrc``、zsh なら ``~/.zshrc``）に ``PATH`` の行を追加するか尋ねる。
``y`` と答えるか、最初から ``--add-path`` を付けると、次の行が追記される。
``n`` と答えたときは、画面に表示される同じ行を自分で設定ファイルに書き加える。

.. code-block:: sh

   export PATH="$HOME/.local/bin:$PATH"

設定ファイルを書き換えたあとは、**新しい端末を開く**\ （または ``source ~/.bashrc`` を実行する）。

.. _sec-install-check:

動作の確認
~~~~~~~~~~

新しい端末で、バージョンを表示し、例題を実行する（:numref:`fig-screen-install-check`）。

.. code-block:: sh

   ignisyeet --version
   cp -r ~/.local/share/ignisyeet/current/examples ignisyeet-examples
   cd ignisyeet-examples
   ignisyeet sim sample.toml
   ignisyeet-plot all out

バージョンの 3 行が出て、``sim`` が結果パネルを出し、``out/`` にファイルができ、``out/plots/`` に図ができれば成功である。
結果の値と手順ごとの出力は :ref:`page-tutorial` の章にある。

.. _fig-screen-install-check:

.. figure:: /_generated/screens/screen_install_check.png
   :width: 100%
   :alt: ignisyeet --version の出力と例題のコピー

   ``ignisyeet --version`` と、例題のコピーおよび実行の入力である。

developer として入れる
----------------------

役割の質問で ``2`` を選ぶ（または ``--developer`` を付ける）と、リリースの実行ファイルではなく、ソースからビルドして使う環境ができる。
その他の手順は user と同じである。

* git でリポジトリを取得する（すでに clone があれば再利用し、変更がなければ ``git pull`` する）。
* ``cargo`` がなければ ``rustup`` を入れ、``cargo build --release`` を実行する。``cargo test --release`` を実行するかも尋ねられる。
* 作図用の環境とドキュメント用の環境を作る。
* TeX Live、dvisvgm、mutool、inkscape、IPAex フォントのうち足りないものは、実行すべき ``apt`` または ``brew`` のコマンドを表示する（``sudo`` はインストーラ自身は実行しない）。

実行する内容は ``--dry-run`` で先に確かめられる（:numref:`fig-screen-install`）。何も変更せずに、検出したツールと実行する手順の一覧だけが出る。

.. _fig-screen-install:

.. figure:: /_generated/screens/screen_install.png
   :width: 100%
   :alt: install.sh の実行画面（developer、CFD あり、--dry-run）

   ``install.sh --developer --cfd --dry-run`` の画面。検出したツールと実行する手順の一覧だけを示す。

ソースからビルドする
~~~~~~~~~~~~~~~~~~~~

インストーラを使わずに、リポジトリから直接ビルドすることもできる。

.. code-block:: sh

   git clone https://github.com/ddd3h/ignisyeet.git
   cd ignisyeet
   cargo build --release

``target/release/ignisyeet`` ができる。
CFD モード（``aero.method = "cfd"``）は、Unix 系の OS（Linux、macOS、Windows の WSL）が前提である。
CFD 以外の部分は Windows でも ``cargo check`` が通るが、インストーラは Linux と macOS 向けである。

.. _sec-python-env:

``pip`` で作図環境を作る
~~~~~~~~~~~~~~~~~~~~~~~~~~

``uv`` を使わずに ``pip`` で作図する場合は、仮想環境を作って依存パッケージを入れ、``python`` から直接実行する（Python 3.10 以上）。
リポジトリの最上位のディレクトリで、先に ``ignisyeet sim examples/sample.toml`` を実行してから、次を実行する。図は ``examples/out/plots/`` に書かれる。

.. code-block:: sh

   python3 -m venv .venv && . .venv/bin/activate
   pip install -r python/requirements.txt -c scripts/plot-constraints.txt
   python python/plot.py all examples/out

CFD ツールと SU2
----------------

CFD ツール（SU2、gmsh、Open MPI。STL の壁面に使う manifold3d を含む）は、どちらの役割でも任意で入れられる。
conda-forge から取得するので約 3 GB のダウンロードがあり、既存の micromamba、mamba、conda と既存の ``ignisyeet-cfd`` 環境があればそれを使う。
パッケージの版は、すべてのプラットフォームで SU2 8.3.0、gmsh 4.15.2、Open MPI 5 に固定される。
入れるには、``--cfd`` を付けて実行するか、上の質問 2 に ``y`` と答える。

.. code-block:: sh

   curl -fsSL https://raw.githubusercontent.com/ddd3h/ignisyeet/main/install.sh | bash -s -- --cfd

終わりに表示される ``[aero.cfd] prefix`` に書く値（または環境変数 ``IGNISYEET_CFD_PREFIX``）を控える。
インストーラが入れた SU2 は ``ignisyeet`` が自動で見つけるので、通常は設定の変更は要らない。
CFD ツールの確認と設定は :ref:`page-howto-cfd-setup` にある。

.. _sec-installer-su2:

SU2 の選択とソース構築
~~~~~~~~~~~~~~~~~~~~~~

Linux では、``--cfd`` は SU2 を実行する CPU 向けにソースから構築する（3〜10 分。12 コアのデスクトップ CPU で 2 分 40 秒）。
構築には GitHub への接続が要り、CFD 環境の gcc/g++ を使う。ディスクは構築中に約 0.5 GB、構築後は ``PREFIX/share/ignisyeet-su2-native`` に約 70 MB が残る。
ここにある ``build-info.txt`` に、コンパイラの版と CPU のモデルが記録される。macOS では、構築せず conda-forge の SU2 を使う。
構築の並列数は ``IGNISYEET_SU2_JOBS`` で決める（既定は全コア数で、利用可能なメモリ 2 GB につき 1 並列が上限である）。

構築した SU2 は、構築した CPU と同じ種類の CPU でだけ動く。別の CPU に移したら、その上で ``install.sh --cfd --force`` を実行し直す。
CPU の世代が混在する共有クラスタやコンテナでは、``--su2-binary`` を付けて構築を省き、移植できる conda-forge の SU2 を使う。
このとき、すでにある ``share/ignisyeet-su2-native`` は削除される。
ソースからの構築が失敗したときは警告（:ref:`IY-I540`、:ref:`IY-I541`）を出して、conda-forge の SU2 を使い続ける。

CFD の計算で使う SU2 は、次の順で最初に見つかったものである。

#. ``[aero.cfd]`` の ``su2``\ （既定の ``SU2_CFD`` 以外を指定したとき）
#. 環境変数 ``IGNISYEET_SU2_CFD``
#. 構築した ``share/ignisyeet-su2-native/bin/SU2_CFD``\ （インストール先、``IGNISYEET_PREFIX``、``~/.local`` の順に探す）
#. ``[aero.cfd]`` の ``prefix`` と ``PATH`` にある ``SU2_CFD``\ （conda-forge のバイナリ）

``mpirun``、gmsh、ライブラリは、どの場合も ``[aero.cfd]`` の ``prefix`` の CFD 環境のものを使う。
``--prefix`` で既定以外のインストール先にしたときは、実行時に ``IGNISYEET_PREFIX`` を同じ場所に設定しないと、構築した SU2 が見つからない。

.. _page-reference-installer:

オプションと環境変数
--------------------

.. _sec-installer-options:

オプション
~~~~~~~~~~

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
     - ``--cfd`` と併せて、SU2 のソース構築を省いて conda-forge の SU2 を使う。
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
     - 削除する（:ref:`sec-installer-uninstall`）。
   * - ``--purge``
     - ``--uninstall`` と併せて、作図環境、CFD 環境、SU2 のソース構築を確認なしに削除する。
   * - ``-h`` ``--help``
     - オプションの一覧を表示して終了する。

.. _sec-installer-env:

環境変数
~~~~~~~~

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
     - SU2 のソース構築の並列数
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

.. _sec-installer-update:

更新と再インストール
--------------------

インストール用のコマンドをもう一度実行すると更新になる。入っていないものだけを入れるので、再実行しても安全である。
user では最新のリリースが取得され、``current`` のリンクが新しい版に切り替わり、古い版のデータは ``~/.local/share/ignisyeet/<バージョン>`` に残る。
特定のバージョンは ``--version vX.Y.Z`` で指定する。
同じバージョンがすでに入っているときは ``Reinstall it?`` と尋ねる。尋ねない場面（``-y`` など）で入れ直すには ``--force`` を付ける。

.. _sec-installer-uninstall:

アンインストール
----------------

``bash install.sh --uninstall`` を実行する。
バイナリ、``ignisyeet-plot`` のリンク、リリースデータを削除するか尋ねたあと、作図用の環境、CFD の conda 環境と SU2 のソース構築（作成していた場合）を削除するかをそれぞれ尋ねる（:numref:`fig-screen-install-uninstall`）。
developer の clone は、明示的に確認したときだけ削除する。
``--purge`` を付けると、作図環境、CFD 環境、SU2 のソース構築も尋ねずに削除する。
``install.log`` と、シェルの設定ファイルに追記した ``PATH`` の行は残る。

.. _fig-screen-install-uninstall:

.. figure:: /_generated/screens/screen_install_uninstall.png
   :width: 100%
   :alt: install.sh --uninstall の実行画面

   ``install.sh --uninstall`` の画面。バイナリとリリースデータ、作図環境の削除に ``y`` と答えたところである。

.. _sec-installer-symptoms:

うまくいかないとき
------------------

失敗した手順の名前とログの末尾が画面に出る。全体の記録は ``~/.local/share/ignisyeet/install.log`` にある。インストーラは再実行しても安全なので、原因を取り除いてからもう一度実行する。
個別の症状は次のとおりである。コードの意味は :ref:`errors-install` にあり、エラーとワーニング全般の読み方は :ref:`page-howto-warnings-errors` にある。

``ignisyeet: command not found``
   ``~/.local/bin`` が ``PATH`` に入っていない。「PATH を設定する」の ``export PATH=...`` の行をシェルの設定ファイルに書き、新しい端末を開く。

最新のリリースが分からない（``could not determine the latest release``、:ref:`IY-I021`）
   まだリリースが公開されていないか、ネットワークに接続できていない。``--developer`` で入れるか、``--version vX.Y.Z`` で版を指定する。

プロキシの内側でダウンロードに失敗する（:ref:`IY-I042`）
   ``curl`` は ``https_proxy`` と ``http_proxy`` を読む。実行前に設定しておく（例：``export https_proxy=http://proxy.example.com:8080``）。

チェックサムが一致しない（:ref:`IY-I044`）
   取得したファイルが壊れているか、配布物と異なる。インストーラは何もインストールしないので、時間をおいてもう一度実行する。

CFD ツールの SU2 が異常終了する（:ref:`IY-I510`）
   AVX-512 のない CPU では SU2 8.5.0 が異常終了する。インストーラが作る環境は SU2 8.3.0 に固定している。別の版が入った環境は、画面に出る再作成のコマンドで作り直せる。原因と手順は :ref:`IY-I510` にある。
