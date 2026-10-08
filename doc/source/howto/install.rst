.. _page-howto-install:

インストール
==============

IgnisYeet をインストールし、動作を確かめる。インストーラを使う方法を中心に、developer の入れ方、CFD ツール、ソースからのビルド、``pip`` による作図環境も扱う。
オプションと環境変数の一覧、アンインストールは :ref:`page-reference-installer` にある。

目的
------

* 計算に使う ``ignisyeet`` コマンドと、作図に使う ``ignisyeet-plot`` コマンドを使えるようにする。
* 必要なら CFD ツール（SU2、gmsh、Open MPI）も入れる。

前提
------

* 対応する OS：Linux（x86_64、aarch64）または macOS（Intel、Apple Silicon）。
* ``bash``、``curl``、``tar``\ （ほとんどの環境に最初から入っている）。
* インターネット接続（リリースの取得と、作図用ライブラリの導入に使う）。
* 管理者権限（``sudo``）は要らない。すべて自分のホームディレクトリの下（既定は ``~/.local``）に入る。

``git``、``cargo``、``uv``、``python3`` などは、user では必須ではない。
``python3`` が 3.10 未満または無いときは、インストーラが ``uv`` を使って用意することを提案する。

手順
------

A. インストーラで入れる（user）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

1. 次の 1 行を端末に貼り付けて実行する。

   .. code-block:: sh

      curl -fsSL https://raw.githubusercontent.com/ddd3h/ignisyeet/main/install.sh | bash

   インターネット上のスクリプトをそのまま実行することに抵抗があるときは、いったん保存して中身を読んでから実行するとよい。

   .. code-block:: sh

      curl -fsSLO https://raw.githubusercontent.com/ddd3h/ignisyeet/main/install.sh
      less install.sh
      bash install.sh

   期待する結果：ロゴが出たあと、質問が始まる。``curl ... | bash`` で実行しても、質問は端末から読み取るのでそのまま答えられる。

2. 役割を選ぶ（:numref:`fig-screen-install-role`）。``1`` が user（既定）、``2`` が developer である。Enter だけで user になる。

   user
      GitHub のリリースから自分のプラットフォーム用の実行ファイルを取得し、チェックサムを検証してから ``~/.local/bin/ignisyeet`` に置く。
      例題と ``python/plot.py`` は ``~/.local/share/ignisyeet/<バージョン>`` に入り、``current`` というリンクが最新版を指す。
      作図用の環境と、それを使う ``ignisyeet-plot`` コマンドも用意する。

   developer
      ソースからビルドして開発する人向けである（下の B）。

   期待する結果：選んだ役割が画面に表示され、次の質問に進む。

   .. _fig-screen-install-role:

   .. figure:: /_generated/screens/screen_install_role.png
      :width: 100%
      :alt: インストーラの起動画面と役割の質問

      インストーラの起動画面。``1``\ （user）を選んだところである。

3. CFD ツールを入れるか選ぶ（:numref:`fig-screen-install-cfd`）。
   インストーラは、すでに入っているツール（cargo、uv、python3、git、conda や micromamba、既存の ``ignisyeet-cfd`` 環境、SU2、gmsh など）を検出して ``Environment`` に一覧にする。
   続けて、CFD ツールを入れるかを尋ねる。既定は ``n``\ （入れない）である。
   CFD ツールは CFD による空力計算で使う任意の部品で、Barrowman 法やパネル法だけを使うなら要らない（入れ方は下の C）。
   すでに SU2 が見つかっているときは質問しない。質問を省くには ``--cfd`` または ``--no-cfd`` を指定する。

   期待する結果：``Environment`` の行頭に、見つかったもの（緑のチェック）と見つからなかったもの（黄色の丸）が並ぶ。

   .. _fig-screen-install-cfd:

   .. figure:: /_generated/screens/screen_install_cfd.png
      :width: 100%
      :alt: 環境の検出結果と CFD ツールの質問

      検出したツールの一覧と CFD ツールの質問。``n`` と答えたところである。

4. 計画を確認する（:numref:`fig-screen-install-plan`）。
   実行する内容が ``Plan`` として一覧になり、``Proceed?`` と尋ねられる。この時点では何も変更していないので、内容が気に入らなければ ``n`` で中止できる。
   ``y`` または Enter で進む。

   期待する結果：``Plan`` に、取得するファイル名、置き場所、作る環境が順番に並ぶ。

   .. _fig-screen-install-plan:

   .. figure:: /_generated/screens/screen_install_plan.png
      :width: 100%
      :alt: 実行計画の一覧と確認

      実行計画と確認。``y`` と答えて実行に進む。

5. インストールの進行を待つ（:numref:`fig-screen-install-progress`）。
   各手順にスピナー（回転する記号）が付き、ダウンロードでは進み具合を示すバーとサイズが出る。
   終わった手順には緑のチェックと所要時間が付く。
   作図用環境の作成は、初回はライブラリ（numpy、pandas、matplotlib）の取得のため数十秒かかることがある。
   詳しい出力はすべて ``~/.local/share/ignisyeet/install.log`` に記録される。

   期待する結果：すべての手順に緑のチェックが付く。

   .. _fig-screen-install-progress:

   .. figure:: /_generated/screens/screen_install_progress.png
      :width: 100%
      :alt: ダウンロード中の進行バー

      ダウンロード中の画面。進行バーと、取得済みのサイズ・全体のサイズが出る。

.. _sec-install-path:

PATH を設定する
~~~~~~~~~~~~~~~~~

実行ファイルを置く ``~/.local/bin`` が ``PATH`` に入っていないときは、インストーラがシェルの設定ファイル（bash なら ``~/.bashrc``、zsh なら ``~/.zshrc``）に ``PATH`` の行を追加するか尋ねる。
``y`` と答えるか、最初から ``--add-path`` を付けると、次の行が追記される。
``n`` と答えたときは、画面に表示される同じ行を自分で設定ファイルに書き加える。

.. code-block:: sh

   export PATH="$HOME/.local/bin:$PATH"

設定ファイルを書き換えたあとは、**新しい端末を開く**\ （または ``source ~/.bashrc`` を実行する）。

期待する結果：最後に ``IgnisYeet is ready`` の枠が出て、インストールされたバージョン、実行ファイル、データ、作図環境の場所と、次に実行するコマンドが書かれている（:numref:`fig-screen-install-done` は :ref:`page-tutorial` にある）。

B. developer として入れる
~~~~~~~~~~~~~~~~~~~~~~~~~~~

役割の質問で ``2`` を選ぶ（または ``--developer`` を付ける）と、リリースの実行ファイルではなく、ソースからビルドして使う環境ができる。
その他の手順は user と同じである。

* git でリポジトリを取得する（すでに clone があれば再利用し、変更がなければ ``git pull`` する）。
* ``cargo`` がなければ ``rustup`` を入れ、``cargo build --release`` を実行する。``cargo test --release`` を実行するかも尋ねられる。
* 作図用の環境とドキュメント用の環境を作る。
* TeX Live、dvisvgm、mutool、inkscape、IPAex フォントのうち足りないものは、実行すべき ``apt`` または ``brew`` のコマンドを表示する（``sudo`` はインストーラ自身は実行しない）。

実行する内容は ``--dry-run`` で先に確かめられる（:numref:`fig-screen-install`）。

期待する結果：何も変更せずに、検出したツールと実行する手順の一覧だけが出る。

.. _fig-screen-install:

.. figure:: /_generated/screens/screen_install.png
   :width: 100%
   :alt: install.sh の実行画面（developer、CFD あり、--dry-run）

   ``install.sh --developer --cfd --dry-run`` の画面。検出したツールと実行する手順の一覧だけを示す。

C. CFD ツールを入れる
~~~~~~~~~~~~~~~~~~~~~~~

どちらの役割でも、A の手順 3 で CFD ツール（SU2、gmsh、Open MPI。STL の壁面に使う manifold3d を含む）を任意で入れられる。
conda-forge から取得するので約 3 GB のダウンロードがあり、既存の micromamba、mamba、conda と既存の ``ignisyeet-cfd`` 環境があればそれを使う。
パッケージの版は、すべてのプラットフォームで SU2 8.3.0、gmsh 4.15.2、Open MPI 5 に固定される。

1. ``--cfd`` を付けて実行するか、A の手順 3 の質問に ``y`` と答える。

   .. code-block:: sh

      curl -fsSL https://raw.githubusercontent.com/ddd3h/ignisyeet/main/install.sh | bash -s -- --cfd

   期待する結果：Linux では、SU2 8.3.0 を実行する CPU 向けにソースから構築する（3〜10 分。12 コアのデスクトップ CPU で 2 分 40 秒）。構築には GitHub への接続が要り、ディスクは構築中に約 0.5 GB、構築後は 70 MB である。macOS では、構築せず conda-forge の SU2 を使う。

2. 終わりに表示される ``[aero.cfd] prefix`` に書く値（または環境変数 ``IGNISYEET_CFD_PREFIX``）を控える。
   インストーラが入れた SU2 は ``ignisyeet`` が自動で見つけるので、通常は設定の変更は要らない。

構築した SU2 は、構築した CPU に固有である。別の CPU に移したら、その上で ``install.sh --cfd --force`` を実行し直す。
CPU の世代が混在する共有クラスタやコンテナでは、``--su2-binary`` を付けて構築を省き、移植できる conda-forge の SU2 を使う。
ソースからの構築が失敗したときは警告（:ref:`IY-I540`、:ref:`IY-I541`）を出して、conda-forge の SU2 を使い続ける。
CFD ツールの確認と設定は :ref:`page-howto-cfd-setup` にある。

D. ソースから入れる
~~~~~~~~~~~~~~~~~~~~~

インストーラを使わずに、リポジトリから直接ビルドすることもできる。

.. code-block:: sh

   git clone https://github.com/ddd3h/ignisyeet.git
   cd ignisyeet
   cargo build --release

期待する結果：``target/release/ignisyeet`` ができる。

CFD モード（``aero.method = "cfd"``）は、Unix 系の OS（Linux、macOS、Windows の WSL）が前提である。
CFD 以外の部分は Windows でも ``cargo check`` が通るが、インストーラは Linux と macOS 向けである。

.. _sec-python-env:

E. ``pip`` で作図環境を作る
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

``uv`` を使わずに ``pip`` で作図する場合は、仮想環境を作って依存パッケージを入れ、``python`` から直接実行する（Python 3.10 以上）。
リポジトリの最上位のディレクトリで実行する。

.. code-block:: sh

   python3 -m venv .venv && . .venv/bin/activate
   pip install -r python/requirements.txt -c scripts/plot-constraints.txt
   python python/plot.py all examples/out

期待する結果：図が ``examples/out/plots/`` に書かれる（先に ``ignisyeet sim examples/sample.toml`` を実行しておく）。

.. _sec-install-check:

確認
------

新しい端末で、バージョンを表示し、例題を実行する（:numref:`fig-screen-install-check`）。

.. code-block:: sh

   ignisyeet --version
   cp -r ~/.local/share/ignisyeet/current/examples ignisyeet-examples
   cd ignisyeet-examples
   ignisyeet sim sample.toml
   ignisyeet-plot all out

期待する結果：バージョンの 3 行が出て、``sim`` が結果パネルを出し、``out/`` にファイルができ、``out/plots/`` に図ができる。
サンプルの結果は、頂点 4440.5 m、着地点が東 −655.6 m、北 −7707.7 m である。
手順ごとの出力は :ref:`page-tutorial` にある。

.. _fig-screen-install-check:

.. figure:: /_generated/screens/screen_install_check.png
   :width: 100%
   :alt: ignisyeet --version の出力と例題のコピー

   ``ignisyeet --version`` と、例題のコピーおよび実行の入力である。

更新するには、A の手順 1 のコマンドをもう一度実行する。入っていないものだけを入れるので、再実行しても安全である。
user では最新のリリースが入り、``current`` のリンクが新しい版に切り替わる。
特定の版を入れるには ``--version vX.Y.Z`` を指定する。

うまくいかないとき
--------------------

``ignisyeet: command not found``
   ``~/.local/bin`` が ``PATH`` に入っていない。「PATH を設定する」の ``export PATH=...`` の行をシェルの設定ファイルに書き、新しい端末を開く。

最新のリリースが分からない（:ref:`IY-I021`）
   まだリリースが公開されていないか、ネットワークに接続できていない。``--developer`` で入れるか、``--version vX.Y.Z`` で版を指定する。

プロキシの内側でダウンロードに失敗する（:ref:`IY-I042`）
   ``curl`` は ``https_proxy`` と ``http_proxy`` を読む。実行前に設定しておく（例：``export https_proxy=http://proxy.example.com:8080``）。

途中で失敗した
   失敗した手順の名前とログの末尾が画面に出る。全体の記録は ``~/.local/share/ignisyeet/install.log`` にある。
   インストーラは再実行しても安全なので、原因を取り除いてからもう一度実行する。

チェックサムが一致しない（:ref:`IY-I044`）
   取得したファイルが壊れているか、配布物と異なる。インストーラは何もインストールしないので、時間をおいてもう一度実行する。

CFD ツールの SU2 が異常終了する（:ref:`IY-I510`）
   AVX-512 のない CPU では、SU2 8.5.0 が異常終了する。インストーラが作る環境は SU2 8.3.0 に固定されているが、別の版がすでに入った環境を使っているときは、画面に出る再作成のコマンドで作り直す。

エラーとワーニングの読み方は :ref:`page-howto-warnings-errors` にある。

関連
------

* :ref:`page-tutorial`：インストールから最初の計算まで
* :ref:`page-reference-installer`：インストーラのオプション、環境変数、アンインストール
* :ref:`page-howto-cfd-setup`：CFD ツールの確認と設定
* :ref:`errors-install`：インストーラのエラーコード
