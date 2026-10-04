使い方
======

必要なもの
----------

.. list-table::
   :header-rows: 1
   :widths: 25 75

   * - ソフトウェア
     - 用途
   * - Rust（stable）と ``cargo``
     - 本体（形状抽出・空力解析・飛翔計算・落下分散）のビルド
   * - `uv <https://docs.astral.sh/uv/>`_ と Python 3.10 以上（uv の代わりに ``venv`` と ``pip`` でもよい）
     - 作図（numpy, pandas, matplotlib）。uv なら自動で導入する。pip の場合は ``python/requirements.txt`` を使う（:ref:`最初の実行の節 <sec-python-env>`）
   * - LuaLaTeX（luatexja）と dvisvgm
     - このドキュメントの TikZ 図を作るときのみ
   * - upLaTeX、dvipdfmx、latexmk と inkscape
     - このドキュメントの PDF 版を作るときのみ（:ref:`sec-doc-build`）

Rust と Python について
~~~~~~~~~~~~~~~~~~~~~~~

IgnisYeet は、計算の重い部分を Rust、作図を Python で書いている。二つの言語の特徴と、この役割分担にした理由を簡単に述べる。

Rust
   コンパイルして機械語の実行ファイルを作る言語で、C や C++ と同程度の速さで動く。
   特徴は、メモリの扱いの誤り（解放済みメモリの参照や、複数のスレッドからの同時書き込みなど）を、
   **実行前のコンパイルの段階で検出する** ことである。ガベージコレクタを使わずにこれを実現するので、速さを損なわない。
   このため、数十万ステップの数値積分や、パネル法の大きな連立一次方程式、
   数千ケースの落下分散を複数の CPU コアで並列に計算する部分を、安全かつ高速に書ける。
   ビルド・テスト・依存ライブラリの管理は付属の ``cargo`` がまとめて行う。
   インストールは公式の ``rustup``\ （https://rustup.rs）を使うのが簡単である。

Python
   コンパイル不要で、そのまま実行できる言語である。文法が簡潔で読みやすく、
   `numpy <https://numpy.org/>`__\ （数値配列）、`pandas <https://pandas.pydata.org/>`__\ （表データ）、`matplotlib <https://matplotlib.org/>`__\ （グラフ）といった科学技術計算向けのライブラリが充実している。
   一方、Python で書いたループは Rust に比べて桁違いに遅い。
   そこで IgnisYeet では、計算結果の CSV を読み込んで図にする部分だけを Python で書いている。
   ライブラリの導入には、標準の ``pip``\ （仮想環境 ``venv`` と組み合わせる）か、
   より高速なパッケージ管理ツール ``uv`` を使う。

計算部分と作図部分は CSV・JSON ファイルを介してつながっているだけなので、
作図を別のツール（Excel、gnuplot、MATLAB など）で行うこともできる。

インストール
------------

インストーラ
~~~~~~~~~~~~

Linux（x86_64、aarch64）と macOS（Intel、Apple Silicon）では、インストーラ ``install.sh`` を実行するだけでインストールできる。
ここでは、利用者（user）として入れる場合の手順を、実際の画面とともに順に説明する。
画面は ``install.sh`` の実際の出力であり、パスはホームディレクトリを ``~`` で示している。

手順 1　準備
^^^^^^^^^^^^

必要なものは次のとおりである。管理者権限（``sudo``）は要らない。すべて自分のホームディレクトリの下（既定は ``~/.local``）に入る。

* 対応する OS（上記の Linux または macOS）
* ``bash``、``curl``、``tar``\ （ほとんどの環境に最初から入っている）
* インターネット接続（リリースの取得と、作図用ライブラリの導入に使う）

``git``、``cargo``、``uv``、``python3`` などは、user では必須ではない。
``python3`` が 3.10 未満または無いときは、インストーラが ``uv`` を使って用意することを提案する。

手順 2　インストーラを実行する
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

次の 1 行を端末に貼り付けて実行する。

.. code-block:: sh

   curl -fsSL https://raw.githubusercontent.com/ddd3h/ignisyeet/main/install.sh | bash

インターネット上のスクリプトをそのまま実行することに抵抗があるときは、いったん保存して中身を読んでから実行するとよい。

.. code-block:: sh

   curl -fsSLO https://raw.githubusercontent.com/ddd3h/ignisyeet/main/install.sh
   less install.sh
   bash install.sh

インストーラは起動すると、ロゴを表示してから質問を始める。
``curl ... | bash`` で実行しても、質問は端末から読み取るので、そのまま答えられる。

手順 3　役割を選ぶ
^^^^^^^^^^^^^^^^^^

最初に、どのように使うかを尋ねられる（:numref:`fig-screen-install-role`）。
``1`` が user（既定）、``2`` が developer である。Enter だけを押すと user になる。

user
   GitHub のリリースから自分のプラットフォーム用のバイナリを取得し、SHA-256 を検証してから
   ``~/.local/bin/ignisyeet`` に置く。例題と ``python/plot.py`` は ``~/.local/share/ignisyeet/<バージョン>`` に入り、
   ``current`` というリンクが最新版を指す。作図用の環境（uv、なければ ``venv`` と ``pip``）と、
   それを使う ``ignisyeet-plot`` コマンドも用意する。ドキュメント作成用のツールは入れない。

developer
   ソースからビルドして開発する人向けである。詳しくは後の「developer の場合」で述べる。

.. _fig-screen-install-role:

.. figure:: _generated/screens/screen_install_role.png
   :width: 100%
   :alt: インストーラの起動画面と役割の質問

   手順 3：インストーラの起動画面。``1``\ （user）を選んだところである。

手順 4　CFD ツールを入れるか選ぶ
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

インストーラは、すでに入っているツール（cargo、uv、python3、git、conda や micromamba、既存の ``ignisyeet-cfd`` 環境、SU2、gmsh など）を検出して
一覧にする（``Environment``）。続けて、CFD ツール（SU2、gmsh、Open MPI。conda-forge から約 3 GB）を入れるかを尋ねられる
（:numref:`fig-screen-install-cfd`）。これは CFD による空力計算（:doc:`cfd` を参照）で使う任意の部品で、
Barrowman 法やパネル法だけを使うなら要らない。既定は ``n``\ （入れない）である。
既存の micromamba、mamba、conda と既存の ``ignisyeet-cfd`` 環境があればそれを使い、すでに SU2 が見つかっているときは質問しない。
質問を省くには ``--cfd`` または ``--no-cfd`` を指定する。

.. _fig-screen-install-cfd:

.. figure:: _generated/screens/screen_install_cfd.png
   :width: 100%
   :alt: 環境の検出結果と CFD ツールの質問

   手順 4：検出したツールの一覧と CFD ツールの質問。``n`` と答えたところである。

手順 5　計画を確認する
^^^^^^^^^^^^^^^^^^^^^^

答えがそろうと、実行する内容が ``Plan`` として一覧になり、``Proceed?`` と尋ねられる（:numref:`fig-screen-install-plan`）。
この時点では何も変更していないので、内容が気に入らなければ ``n`` で中止できる。

``Environment`` の行頭の記号は、次の意味である。

* 緑のチェック（✔）：すでに見つかった。入っているものは再インストールせず、そのまま使う。
* 黄色の丸（○）：見つからなかった。必要なものは ``Plan`` で入れる手順に含まれる。

``Plan`` には、取得するファイル名、置き場所、作る環境が順番に並ぶ。
``y`` または Enter で進む。

.. _fig-screen-install-plan:

.. figure:: _generated/screens/screen_install_plan.png
   :width: 100%
   :alt: 実行計画の一覧と確認

   手順 5：実行計画と確認。``y`` と答えて実行に進む。

手順 6　インストールの進行を見る
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

各手順は、スピナー（回転する記号）とともに実行される。ダウンロードでは、進み具合を示すバーとサイズが出る
（:numref:`fig-screen-install-progress`）。続いてチェックサムの検証、展開、作図用環境の作成が行われ、
終わった手順には緑のチェックと所要時間が付く。
詳しい出力はすべて ``~/.local/share/ignisyeet/install.log`` に記録される。
作図用環境の作成は、初回はライブラリ（numpy、pandas、matplotlib）の取得のため、数十秒かかることがある。

.. _fig-screen-install-progress:

.. figure:: _generated/screens/screen_install_progress.png
   :width: 100%
   :alt: ダウンロード中の進行バー

   手順 6：ダウンロード中の画面。進行バーと、取得済みのサイズ・全体のサイズが出る。

手順 7　完了と PATH の設定
^^^^^^^^^^^^^^^^^^^^^^^^^^

最後に、結果をまとめた枠（``IgnisYeet is ready``）が出る（:numref:`fig-screen-install-done`）。
インストールされたバージョン、実行ファイル、データ、作図環境の場所と、次に実行するコマンドが書かれている。

実行ファイルを置く ``~/.local/bin`` が ``PATH`` に入っていないときは、インストーラがシェルの設定ファイル
（bash なら ``~/.bashrc``、zsh なら ``~/.zshrc``）に PATH の行を追加するか尋ねる。
``y`` と答えるか、最初から ``--add-path`` を付けると、次の行が追記される。
``n`` と答えたときは、枠の下に表示される行を、自分で設定ファイルに書き加える。

.. code-block:: sh

   export PATH="$HOME/.local/bin:$PATH"

設定ファイルを書き換えたあとは、**新しい端末を開く**\ （または ``source ~/.bashrc`` を実行する）と、``ignisyeet`` コマンドが使えるようになる。

.. _fig-screen-install-done:

.. figure:: _generated/screens/screen_install_done.png
   :width: 100%
   :alt: インストール完了の枠と PATH の案内

   手順 7：完了の画面。PATH の質問（ここでは ``n``）、結果の枠、``PATH`` に追加する行が出る。

手順 8　動作を確認する
^^^^^^^^^^^^^^^^^^^^^^

新しい端末で、バージョンを表示して確かめる。

.. code-block:: sh

   ignisyeet --version

続いて、インストールされた例題を作業用の場所にコピーし、計算を実行する。
計算が終わると ``out/`` に結果の CSV と JSON が書かれる（:numref:`fig-screen-install-check`、:numref:`fig-screen-install-check-sim`）。

.. code-block:: sh

   cp -r ~/.local/share/ignisyeet/current/examples ignisyeet-examples
   cd ignisyeet-examples
   ignisyeet sim sample.toml

図を描くには、``ignisyeet-plot`` を使う。作図用の環境はインストーラが用意したものが自動で使われる。

.. code-block:: sh

   ignisyeet-plot all out

.. _fig-screen-install-check:

.. figure:: _generated/screens/screen_install_check.png
   :width: 100%
   :alt: ignisyeet --version の出力と例題の実行

   手順 8：``ignisyeet --version`` と、例題のコピーと実行。

.. _fig-screen-install-check-sim:

.. figure:: _generated/screens/screen_install_check_sim.png
   :width: 100%
   :alt: インストールした例題の計算結果

   手順 8：``ignisyeet sim sample.toml`` の出力。計算の流れ、飛翔の結果、書き出したファイルが出る。

計算全体の流れは、この後の「最初の実行」を参照されたい。

developer の場合
^^^^^^^^^^^^^^^^

手順 3 で ``2``\ （developer）を選ぶと、手順 6 の内容が次のように変わる。
その他の手順は user と同じである。

* git でリポジトリを取得する（既存の clone があれば再利用し、変更がなければ ``git pull`` する）。
* ``cargo`` がなければ ``rustup`` を入れ、``cargo build --release`` と ``cargo test --release`` を実行する（テストは実行するか尋ねられる）。
* ``python/`` と ``doc/`` の環境を ``uv sync`` で作る。
* TeX Live、dvisvgm、mutool、inkscape、IPAex フォントのうち足りないものは、
  実行すべき ``apt`` または ``brew`` のコマンドを表示する（``sudo`` はインストーラ自身は実行しない）。

実行する内容は ``--dry-run`` で先に確かめられる（:numref:`fig-screen-install`）。

.. _fig-screen-install:

.. figure:: _generated/screens/screen_install.png
   :width: 100%
   :alt: install.sh の実行画面（developer、CFD あり、--dry-run）

   ``install.sh --developer --cfd --dry-run`` の画面。何も変更せずに、検出したツールと実行する手順の一覧だけを示す。

CFD ツールについて
^^^^^^^^^^^^^^^^^^

どちらの役割でも、CFD ツール（SU2、gmsh、Open MPI）を任意で入れられる（手順 4）。
x86_64 の Linux で AVX-512 のない CPU では、SU2 8.5.0 のパッケージが異常終了するため、SU2 を 8.3.0 に固定する。
インストール後に、設定ファイルの ``[aero.cfd] prefix`` に書く値（または環境変数 ``IGNISYEET_CFD_PREFIX``）を表示する。

オプション
^^^^^^^^^^

質問に答える代わりに、オプションで指定することもできる。
``curl ... | bash`` の場合は、オプションを ``bash -s --`` の後ろに付ける。

.. code-block:: sh

   curl -fsSL https://raw.githubusercontent.com/ddd3h/ignisyeet/main/install.sh | bash -s -- --developer --cfd -y

.. list-table::
   :header-rows: 1
   :widths: 30 70

   * - オプション
     - 内容
   * - ``--user`` ``--developer``
     - 役割を尋ねずに指定する
   * - ``--cfd`` ``--no-cfd``
     - CFD ツールを入れる、入れない
   * - ``--version vX.Y.Z``
     - 入れるリリース（既定は最新）。developer ではチェックアウトするタグ
   * - ``--prefix DIR``
     - インストール先（既定は ``~/.local``）
   * - ``--dir DIR``
     - developer の clone 先（既定は実行した clone、なければ ``~/ignisyeet``）
   * - ``--add-path``
     - ``PATH`` の行をシェルの設定ファイルに、尋ねずに追記する
   * - ``--force``
     - 同じバージョンが入っていても入れ直す
   * - ``-y`` ``--yes``
     - 質問せず既定値を使う（user、CFD なし、シェル設定ファイルは書き換えない）
   * - ``--dry-run``
     - 計画だけを表示して終了する
   * - ``--uninstall``
     - バイナリ、リンク、リリースデータを削除する。作図環境と CFD 環境は確認してから削除する（``--purge`` で両方）。developer の clone は明示的に確認したときだけ削除する

環境変数
^^^^^^^^

各オプションには環境変数も対応する。

.. list-table::
   :header-rows: 1
   :widths: 40 60

   * - 環境変数
     - 対応するオプションなど
   * - ``IGNISYEET_ROLE``
     - ``--user`` ``--developer``\ （``user`` または ``developer``）
   * - ``IGNISYEET_VERSION``
     - ``--version``
   * - ``IGNISYEET_PREFIX``
     - ``--prefix``
   * - ``IGNISYEET_DIR``
     - ``--dir``
   * - ``IGNISYEET_CFD``
     - ``--cfd``\ （``1``）、``--no-cfd``\ （``0``）
   * - ``IGNISYEET_YES``、``IGNISYEET_DRY_RUN``、``IGNISYEET_FORCE``、``IGNISYEET_ADD_PATH``
     - ``-y``、``--dry-run``、``--force``、``--add-path``\ （``1`` で有効）
   * - ``IGNISYEET_NO_COLOR``
     - 色を使わない（``NO_COLOR`` を設定しても同じになる）
   * - ``IGNISYEET_DOWNLOAD_BASE``、``IGNISYEET_REPO_URL``
     - リリースの取得元の URL、developer が clone する git の URL（ミラーや検証用）

``NO_COLOR`` を設定するか、端末でない場所へ出力すると、装飾のない行単位の出力になる。
端末でないとき（``-y`` を付けた場合やパイプ越しで入力がない場合を含む）は質問せず、既定値（user、CFD なし、PATH は書き換えない）を使う。
リリースの配布物は ``ignisyeet-<バージョン>-<ターゲット>.tar.gz`` と ``.sha256`` であり、
インストーラはチェックサムを検証してから展開する。

アンインストール
^^^^^^^^^^^^^^^^

インストーラに ``--uninstall`` を付けて実行する。

.. code-block:: sh

   bash install.sh --uninstall

バイナリ、``ignisyeet-plot`` のリンク、リリースデータを削除するか尋ねたあと、
作図用の環境、CFD の conda 環境（作成していた場合）を削除するかをそれぞれ尋ねる（:numref:`fig-screen-install-uninstall`）。
developer の clone は、明示的に確認したときだけ削除する。作業中の変更が失われるので、通常は ``n`` と答える。
``--purge`` を付けると、作図環境と CFD 環境も尋ねずに削除する。
``install.log`` と、シェルの設定ファイルに追記した PATH の行は残るので、不要なら手で消す。

.. _fig-screen-install-uninstall:

.. figure:: _generated/screens/screen_install_uninstall.png
   :width: 100%
   :alt: install.sh --uninstall の実行画面

   ``install.sh --uninstall`` の画面。バイナリとリリースデータ、作図環境の削除に ``y`` と答えたところである。

更新と再インストール
^^^^^^^^^^^^^^^^^^^^

新しいバージョンに更新するには、手順 2 のコマンドをもう一度実行する。
入っていないものだけを入れるので、再実行しても安全である。
user では最新のリリースが取得され、``current`` のリンクが新しい版に切り替わる。古い版のデータは ``~/.local/share/ignisyeet/<バージョン>`` に残る。
特定のバージョンを入れたいときは ``--version vX.Y.Z`` を指定する。
同じバージョンがすでに入っているときは、``Reinstall it?`` と尋ねられる。尋ねられない場面（``-y`` など）で入れ直したいときは ``--force`` を付ける。

困ったとき
^^^^^^^^^^

``could not determine the latest release``\ （最新のリリースが分からない）
   まだリリースが公開されていないか、ネットワークに接続できていない。developer の役割（``--developer``）で入れるか、
   ``--version vX.Y.Z`` でバージョンを指定する。

``ignisyeet: command not found``
   ``~/.local/bin`` が ``PATH`` に入っていない。手順 7 の ``export PATH=...`` の行をシェルの設定ファイルに書き、新しい端末を開く。

プロキシの内側でダウンロードに失敗する
   ``curl`` は ``https_proxy`` と ``http_proxy`` を読む。実行前に設定しておく（例：``export https_proxy=http://proxy.example.com:8080``）。

途中で失敗した
   失敗した手順の名前と、ログの末尾が画面に出る。全体の記録は ``~/.local/share/ignisyeet/install.log`` にある。
   インストーラは再実行しても安全なので、原因を取り除いてからもう一度実行する。

CFD ツールで SU2 が異常終了する（AVX-512 のない CPU）
   x86_64 の Linux で AVX-512 のない CPU では、SU2 8.5.0 が異常終了する。
   インストーラはこれを検出して 8.3.0 に固定するが、すでに作成された環境を使っているときは、画面に出る再作成のコマンドで作り直す。

リリースとバージョン
~~~~~~~~~~~~~~~~~~~~

IgnisYeet のバージョンは ``Cargo.toml`` の ``[workspace.package] version``\ （``ignisyeet --version`` で表示される値）で決まり、
GitHub では同じ番号のタグ ``vX.Y.Z`` と GitHub Release で管理する。インストーラの user は、
``--version`` を指定しなければ最新のリリースを入れる。

タグ ``v*`` を push すると、GitHub Actions のワークフロー ``release.yml`` が次の処理を行う。

#. タグと ``Cargo.toml`` のバージョンが一致することを確かめる（一致しなければ失敗する）。
#. 4 つのターゲット向けにバイナリを作る。Linux は musl で静的にリンクするので、ディストリビューションによらず動く。

   .. list-table::
      :header-rows: 1
      :widths: 40 30 30

      * - ターゲット
        - ビルド環境
        - テスト
      * - ``x86_64-unknown-linux-musl``
        - Ubuntu（cargo-zigbuild）
        - 実行する
      * - ``aarch64-unknown-linux-musl``
        - Ubuntu（cargo-zigbuild）
        - 実行しない（クロスビルド）
      * - ``x86_64-apple-darwin``
        - macOS（Intel）
        - 実行する
      * - ``aarch64-apple-darwin``
        - macOS（Apple Silicon）
        - 実行する

#. ``scripts/package-release.sh`` で ``ignisyeet-<バージョン>-<ターゲット>.tar.gz`` と SHA-256 を作る。
   中身はバイナリ（``bin/ignisyeet``）、例題、作図スクリプト（``python/plot.py`` と依存関係の一覧）、
   ``LICENSE``、``NOTICE``、``CITATION.cff``、``README.md`` である。
#. GitHub Release を作り、すべての配布物、チェックサム、``install.sh`` を添付する。
   リリースノートは前のタグからのコミットを Conventional Commits の種類（feat、fix、docs など）ごとにまとめて作る。

新しいバージョンを出すときは、``Cargo.toml``\ （と ``CITATION.cff`` などの表記）のバージョンを上げてコミットし、
同じ番号のタグを付けて push する。

.. code-block:: sh

   git tag v0.3.0
   git push origin v0.3.0

ソースからのビルド
~~~~~~~~~~~~~~~~~~

インストーラを使わずに、リポジトリから直接ビルドすることもできる。

.. code-block:: sh

   git clone https://github.com/ddd3h/ignisyeet.git
   cd ignisyeet
   cargo build --release      # target/release/ignisyeet ができる

最初の実行
----------

付属のサンプル機体（直径 100 mm、全長 1.5 m、4 枚フィン）で全工程を実行する。

.. code-block:: sh

   make all

これは次のコマンドを順に実行するのと同じである。

.. code-block:: sh

   ./target/release/ignisyeet aero       examples/sample.toml   # 空力係数表を作る（初回のみ計算）
   ./target/release/ignisyeet sim        examples/sample.toml   # 1 回の飛翔を計算する
   ./target/release/ignisyeet dispersion examples/sample.toml   # 落下分散を計算する
   uv run --project python python/plot.py all examples/out      # 図を描く

``python/plot.py`` は ``geometry`` ``aero`` ``trajectory`` ``dispersion`` のいずれかを選んで描くこともできる。
``dispersion`` は ``dispersion.csv`` があれば風の格子の図（``dispersion.png``）を、
``dispersion_mc.csv`` と ``dispersion_summary.json`` があればモンテカルロの図（``dispersion_mc.png``）を描く（両方あれば両方描く）。
``aero`` に ``--compare <別の出力ディレクトリ>`` を付けると、二つの係数表の :math:`C_{N\alpha}`、:math:`x_{cp}`、:math:`C_{D0}` を Mach 数に対して重ねた図
（``aero_compare.png``。凡例はディレクトリ名）も描く。

.. _sec-python-env:

uv を使わずに ``pip`` で作図する場合は、仮想環境を作って依存パッケージを入れ、``python`` から直接実行する。

.. code-block:: sh

   python3 -m venv .venv && . .venv/bin/activate
   pip install -r python/requirements.txt
   python python/plot.py all examples/out
   # make 経由なら: make plot PLOT="python python/plot.py"

自分の機体で計算するときは、``examples/sample.toml`` をコピーして編集し、
``make all CONFIG=path/to/rocket.toml`` とすればよい。

端末の表示
----------

``ignisyeet`` を端末で実行すると、設定の一覧、進捗バー、結果の要約を色付きの枠で表示する。
表示は標準出力が端末かどうかで切り替わり、パイプやファイルへの出力では装飾のない行単位の出力になる（:ref:`sec-ui-modes`）。

画面の構成
~~~~~~~~~~

起動すると、まずバナー（バージョンと概要）と設定パネル（Configuration）を出す。
設定パネルには、入力ファイル、機体、射点、回収、空力、環境（地球・大気・風）、積分、出力先が項目ごとにまとまって並ぶ。
設定ファイルの内容を実行前に確認できるので、単位や風向の入れ間違いに気づきやすい。

.. figure:: _generated/screens/screen_sim_start.png
   :width: 100%
   :alt: sim の起動画面（バナーと設定パネル）

   ``sim`` の起動画面。水色の見出しで項目が分かれ、設定値は太字で示す。方式名（``barrowman``）は紫で表示する。

その後は工程ごとに、実行中は進捗バーを、終了すると完了行（チェック印と処理名、要約、所要時間）を表示する。
進捗バーは実行中だけ表示し、工程が終わると完了行に置き換わる。

``aero``
   Barrowman 法では係数表の行数を分母にしたバーを出す。パネル法では段階（形状の準備、亜音速、超音速など）ごとに進み具合を示し、
   2 行目に実行中の処理を表示する。経過時間と残り時間の見積り（ETA。黄色）を併記する。保存済みの表を使うときは、バーを出さずに ``cached`` と行数を表示する。

.. figure:: _generated/screens/screen_aero_panel.png
   :width: 100%
   :alt: aero のパネル法の実行中の画面

   ``aero``\ （``method = "panel"``）の実行中。形状の要約に続いて、段階の進み具合（4/9）と ETA、実行中の処理を表示する。

``sim``
   飛翔の段階（``rail`` ``powered`` ``coast`` ``parachute`` など）と、その時点の高度・速度・Mach 数・静安定余裕を 2 行目に更新し続ける。
   バーの分母は目標とする飛行時間で、飛翔の途中で見積りを更新するため、ETA と終点（``~684s`` のような表示）は計算が進むにつれて補正される。

``dispersion``
   終わったケース数のバーに、1 秒あたりの処理数、経過時間、ETA を表示する。2 行目には、ここまでの最大着地距離（弾道落下とパラシュート降下）を示す。
   ``monte_carlo`` ではサンプル数が分母になる。

.. figure:: _generated/screens/screen_dispersion.png
   :width: 100%
   :alt: dispersion（モンテカルロ）の実行中の画面

   ``dispersion``\ （``mode = "monte_carlo"``）の実行中。処理数、1 秒あたりの処理数、ETA と、ここまでの最大着地距離を表示する。

完了すると結果パネルを出す。``sim`` では Flight result パネルにランチャ離脱、最大速度・最大動圧、頂点、着地点などをまとめる。
最後に、書き出したファイルの一覧（パスとサイズ）と、全体の所要時間を表示する。

.. figure:: _generated/screens/screen_sim_result.png
   :width: 100%
   :alt: sim の完了画面（結果パネルと出力ファイル）

   ``sim`` の完了画面。結果パネル、書き出したファイルの一覧、所要時間が並ぶ。

結果パネルの静安定余裕には判定の印が付く。

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
   * - 1.0 cal 未満
     - 赤のバツ印と ``UNSTABLE``
     - 不安定。設計を見直す

.. _sec-ui-modes:

出力モードと環境変数
~~~~~~~~~~~~~~~~~~~~

.. list-table::
   :header-rows: 1
   :widths: 30 70

   * - 条件
     - 動作
   * - 標準出力が端末
     - 上で述べた表示（バナー、パネル、色）。進捗バーは標準エラー出力が端末のときだけ出す。
   * - 標準出力をパイプ・リダイレクトした
     - プレーンモード。バナー、パネル、色、進捗バーを出さず、行単位の出力（``Flight (...)`` や ``rail exit ...`` など）を書く。
       ``ignisyeet sim ... | tee log.txt`` のように記録しても制御文字が混ざらない。
   * - ``-q`` / ``--quiet``
     - エラーと、書き出したファイルのパスだけを表示する。スクリプトから呼ぶときに使う。
   * - ``--no-progress``
     - 進捗バーとスピナーを出さない。枠付きの要約は表示する。ログに残したい場合などに使う。
   * - ``NO_COLOR`` が空でない値で設定されている
     - 色を使わない（枠や記号は残る）。
   * - ``TERM=dumb``
     - 色と Unicode の記号を使わない。
   * - ロケールが UTF-8 でない（``LANG`` ``LC_ALL`` ``LC_CTYPE`` が UTF-8 を指さない、または ``TERM=linux``）
     - 枠線・記号・バーを ASCII 文字（``+ - |``、``ok`` ``x`` ``!``、``#>-``）に置き換える。

.. code-block:: sh

   ./target/release/ignisyeet sim examples/sample.toml -q          # 書き出したパスだけ
   ./target/release/ignisyeet dispersion examples/sample.toml --no-progress
   NO_COLOR=1 ./target/release/ignisyeet aero examples/sample.toml
   ./target/release/ignisyeet sim examples/sample.toml | cat        # プレーンモード

サブコマンド
------------

.. list-table::
   :header-rows: 1
   :widths: 22 78

   * - コマンド
     - 内容
   * - ``geom <config>``
     - STL から形状を抽出し、``geometry.json`` と ``profile.csv`` を書き出す。抽出結果の確認用。
   * - ``aero <config> [--force]``
     - 空力係数表 ``aero_table.csv`` を作る。入力（STL と空力関係の設定）が前回と同じなら保存済みの表を使う。
       ``--force`` を付けると必ず作り直す。
   * - ``sim <config> [--descent ballistic|parachute]``
     - 設定した風で 1 回飛翔させ、``trajectory.csv`` と ``summary.json`` を書き出す。
   * - ``dispersion <config>``
     - ``dispersion.mode`` に従って落下分散を並列に計算する。``"wind_grid"`` では風速 × 風向の全ケースを計算して ``dispersion.csv`` を、
       ``"monte_carlo"`` では各サンプルの結果 ``dispersion_mc.csv`` と統計 ``dispersion_summary.json`` を書き出す。
   * - ``sample-stl <path>``
     - 付属のサンプル機体の STL（mm 単位、ノーズ +z 向き）を書き出す。

``sim`` と ``dispersion`` は、係数表がない場合や入力が変わった場合に自動で ``aero`` を実行する。
入力の同一性は、STL の内容と設定値から計算したハッシュ値（FNV-1a）で判定する。
プログラム本体を更新したときは、係数の計算方法が変わっている可能性があるので ``aero --force`` を実行してほしい。

設定ファイル
------------

設定は `TOML <https://toml.io/ja/v1.0.0>`_ 形式で書く。
相対パスは **設定ファイルが置かれたディレクトリ** を基準に解釈する。
長さの単位は m で、機体上の位置はすべて **ノーズ先端から後方へ測る**。
各項目の既定値と意味を以下に示す（既定値が「必須」のものは省略できない）。

``[output]``

.. list-table::
   :header-rows: 1
   :widths: 27 22 51

   * - 項目
     - 既定値
     - 意味
   * - ``dir``
     - ``"out"``
     - 出力先ディレクトリ
   * - ``kml``
     - ``true``
     - Google Earth 用の KML ファイル ``ignisyeet.kml`` も書き出す（:ref:`sec-kml`）

``[resources]``\ （計算資源。すべて省略できる。結果は変わらないので、空力係数表のハッシュにも含めない）

.. list-table::
   :header-rows: 1
   :widths: 27 22 51

   * - 項目
     - 既定値
     - 意味
   * - ``threads``
     - ``0``
     - 全処理で使う CPU スレッド数の上限。``0`` は使用可能な全スレッド。使用可能数を超える値は警告して使用可能数に切り詰める。分散計算、モンテカルロ、パネル法の行列計算、CFD の MPI ランク数に効く
   * - ``memory_gb``
     - ``0``
     - 重い処理のメモリ予算 [GB]。``0`` は制限なし。パネル法は密行列に必要な量（パネル数 n に対しておよそ 16 n² バイト）が予算を超えると、計算を始める前にエラーで止まる。CFD は予算に収まるよう同時実行ケース数を減らす。負の値はエラー
   * - ``nice``
     - ``0``
     - プロセスの優先度を下げる値（0 から 19）。長時間の計算で他の作業を妨げたくないときに使う。SU2 や ``mpirun`` の子プロセスにも引き継がれる。Windows など Unix 以外では警告して無視する。範囲外の値はエラー

実際に使うスレッド数、メモリ予算、``nice`` は、起動時の設定パネルに表示する（プレーン表示では 1 行で示す）。

``[rocket]``

.. list-table::
   :header-rows: 1
   :widths: 27 22 51
   :class: longtable

   * - 項目
     - 既定値
     - 意味
   * - ``stl``
     - 必須
     - 機体の STL ファイル（ASCII / バイナリのどちらでもよい）
   * - ``stl_scale``
     - ``1.0``
     - STL の単位を m に直す係数。mm で作った STL なら ``0.001``
   * - ``nose_direction``
     - ``"auto"``
     - STL 座標でノーズが向いている方向。``"+x"`` ``"-x"`` ``"+y"`` ``"-y"`` ``"+z"`` ``"-z"`` で指定できる
   * - ``dry_mass``
     - 必須
     - 推進剤を除いた質量 [kg]（空のモータケースを含む）
   * - ``cg_dry``
     - 必須
     - 推進剤を除いたときの重心位置 [m]
   * - ``ixx_dry`` / ``iyy_dry``
     - 必須
     - 推進剤を除いたときのロール / ピッチ・ヨー慣性モーメント [kg m\ :sup:`2`]（``iyy_dry`` は乾燥重心まわり）
   * - ``roughness``
     - ``60e-6``
     - 表面の等価砂粒粗さ [m]。一般的な塗装面で 60 µm 程度
   * - ``fin_le``
     - ``"rounded"``
     - フィン前縁形状: ``"rounded"`` （丸め）または ``"sharp"`` （鋭角）
   * - ``fin_te``
     - ``"square"``
     - フィン後縁形状: ``"square"`` （切り落とし、底面抗力あり）または ``"tapered"``
   * - ``extra_cd``
     - ``0.0``
     - ランチャラグなどの付加的な抗力係数
   * - ``[rocket.fin_override]``
     - なし
     - フィンの自動検出結果を上書きする。``count`` ``root_chord`` ``tip_chord`` ``span`` ``sweep`` ``thickness`` ``x_le_root`` を必要なものだけ書く

``[motor]``

.. list-table::
   :header-rows: 1
   :widths: 27 22 51

   * - 項目
     - 既定値
     - 意味
   * - ``eng``
     - 必須
     - RASP 形式（``.eng``）の推力曲線ファイル
   * - ``aft_x``
     - 機体全長
     - モータ後端の位置 [m]
   * - ``nozzle_exit_diameter``
     - ``0.0``
     - ノズル出口直径 [m]。燃焼中はこの面積だけ底面抗力が減る
   * - ``propellant_mass``
     - ``.eng`` の値
     - 推進剤質量 [kg] を上書きする

``[launch]``

.. list-table::
   :header-rows: 1
   :widths: 27 22 51

   * - 項目
     - 既定値
     - 意味
   * - ``latitude`` / ``longitude``
     - 必須
     - 射点の緯度・経度 [deg]（WGS84）
   * - ``altitude``
     - 必須
     - 射点の標高 [m]
   * - ``rail_length``
     - 必須
     - ランチャレールの長さ [m]
   * - ``elevation_deg``
     - 必須
     - 射角（水平面からの仰角）[deg]
   * - ``azimuth_deg``
     - 必須
     - 射方位（北から時計回り）[deg]

``[recovery]``

.. list-table::
   :header-rows: 1
   :widths: 27 22 51

   * - 項目
     - 既定値
     - 意味
   * - ``enabled``
     - 必須
     - パラシュートを使うか。``false`` のとき落下分散は弾道落下のみ計算する
   * - ``cd_s``
     - 必須
     - パラシュートの抗力面積 :math:`C_D S` [m\ :sup:`2`]
   * - ``delay``
     - 必須
     - 頂点から開傘までの遅れ [s]

``[aero]``

.. list-table::
   :header-rows: 1
   :widths: 27 22 51

   * - 項目
     - 既定値
     - 意味
   * - ``method``
     - ``"barrowman"``
     - 空力係数の求め方。``"barrowman"`` は STL から抽出した形状に対する部品積み上げ法（:doc:`aerodynamics`）、
       ``"table"`` は外部の係数表の読み込み、``"panel"`` はパネル法（:doc:`panel`）、``"cfd"`` は SU2 による CFD（:doc:`cfd`）
   * - ``table``
     - なし
     - ``method = "table"`` のとき必須。係数表の CSV ファイル（形式は後述）
   * - ``mach_min`` / ``mach_max`` / ``mach_step``
     - ``0`` / ``3`` / ``0.02``
     - 係数表の Mach 数の範囲と刻み
   * - ``alpha_max_deg`` / ``alpha_step_deg``
     - ``30`` / ``1``
     - 係数表の迎角の範囲（0 から）と刻み [deg]
   * - ``n_slices``
     - ``600``
     - 形状抽出で STL を切る断面の数
   * - ``fin_threshold``
     - ``0.05``
     - フィン判定のしきい値（最大胴体半径に対する比）
   * - ``extrapolation``
     - ``"linear"``
     - 表の範囲外の扱い: ``"linear"`` （線形外挿）または ``"clamp"`` （端の値で一定）

``[aero.panel]``\ （``method = "panel"`` のときだけ使う）

.. list-table::
   :header-rows: 1
   :widths: 27 22 51

   * - 項目
     - 既定値
     - 意味
   * - ``body_axial`` / ``body_circ``
     - ``80`` / ``48``
     - 胴体のパネルの軸方向・周方向の分割数（軸方向にはフィン根元の翼弦の分割を含む）
   * - ``fin_chord`` / ``fin_span``
     - ``16`` / ``10``
     - フィン片面のパネルの翼弦方向・スパン方向の分割数
   * - ``wake_length``
     - ``20.0``
     - フィン後流の長さ（全長に対する倍率。全長の 0.5 倍以上）
   * - ``tail_radii``
     - ``6.0``
     - 底面を塞ぐ尾部フェアリング（円筒 + 半球）の円筒部の長さ（底面半径の倍数）。0 なら開口（非推奨）
   * - ``subsonic_machs``
     - ``[0, 0.3, 0.5, 0.6, 0.7, 0.8]``
     - 亜音速のパネル解を計算する Mach 数（1 未満。この間は pchip で内挿）
   * - ``transonic``
     - ``[0.8, 1.2]``
     - 遷音速の補間区間の Mach 数の範囲（下端は最後の亜音速解の Mach 数に合わせる）
   * - ``fin_section``
     - ``"biconvex"``
     - フィンの断面形状（現在は両凸のみ）

各項目の意味と、パネル法が出力する ``panel_cp.csv`` は :doc:`panel` で説明する。

``[aero.cfd]``\ （``method = "cfd"`` のときだけ使う。SU2、gmsh、Open MPI が必要で、設定と計算の流れは :doc:`cfd` で説明する）

.. list-table::
   :header-rows: 1
   :widths: 27 22 51
   :class: longtable

   * - 項目
     - 既定値
     - 意味
   * - ``model``
     - ``"euler"``
     - 支配方程式。``"euler"`` は非粘性の Euler 方程式（粘性の抗力項は部品積み上げ法で補う）、``"rans"`` は Spalart–Allmaras モデルの RANS（検証できていない）
   * - ``machs``
     - ``[0.3, 0.6, 0.8, 0.95, 1.1, 1.3, 1.6, 2.0, 2.5, 3.0]``
     - 解く Mach 数（正、昇順、重複なし）
   * - ``alphas_deg``
     - ``[0, 2, 4, 8, 12, 16]``
     - 解く迎角 [deg]（0 以上、昇順、0 を含み、0 より大きい値が 1 つ以上）
   * - ``alpha_mode``
     - ``"mirror"``
     - 迎角の正負と :math:`\alpha=0` のずれの扱い。``"mirror"`` は正負の解の奇関数・偶関数部分を使う（:math:`z` 対称メッシュでは負の迎角は解かない）、
       ``"offset"`` は :math:`\alpha=0` の値を引く、``"single"`` は結果をそのまま使う
   * - ``z_mirror_mesh``
     - ``true``
     - 1/4 領域をメッシュにして :math:`z=0` で鏡映し、:math:`z` について厳密に対称な半モデルのメッシュにする。フィンの配置が :math:`z` について対称でないときは無視する
   * - ``symmetry``
     - ``true``
     - ピッチ面（:math:`y=0`）で切った半モデル。``false`` は現状エラー
   * - ``surface``
     - ``"panel_mesh"``
     - 壁面の形状。抽出した胴体とフィンを OpenCASCADE で再構成する。``"stl"`` はエラー
   * - ``fin_roll_deg``
     - ``0``
     - フィンの組を機軸のまわりに回す角度 [deg]（0 でフィン 2 枚が :math:`y=0` 面に載る、45 で載らない）
   * - ``tail_fairing``
     - ``6.0``
     - Euler のみ。平らな底面の後ろに置く円すい形の尾部フェアリングの長さ（底面半径の倍数）。0 なら平らな底面。RANS は常に平らな底面
   * - ``farfield``
     - ``20.0``
     - 遠方境界の球の半径（全長の倍数、5 以上）
   * - ``wall_size``
     - ``0.002``
     - 壁面の要素の大きさ [m]。小さいほど細かく、セル数と時間が増える
   * - ``yplus``
     - ``1.0``
     - RANS の目標 :math:`y^+`
   * - ``iterations``
     - ``3000``
     - 1 ケースの最大反復数（RANS は 300 で打ち切る）
   * - ``cfl``
     - ``5.0``
     - CFL 数の初期値（適応的に変える）
   * - ``convergence``
     - ``1e-6``
     - 密度の二乗平均残差の目標（0 と 1 の間）
   * - ``scheme``
     - ``"roe"``
     - 対流フラックス。``"roe"``\ （MUSCL と Venkatakrishnan の制限関数で 2 次精度）または ``"jst"``
   * - ``prefix``
     - ``""``
     - CFD ツールの環境（``<prefix>/bin/SU2_CFD``、``mpirun``、gmsh を持つ ``python``）。空なら環境変数 ``IGNISYEET_CFD_PREFIX``、それも空なら ``PATH``。先頭の ``~`` は展開する
   * - ``su2`` / ``mpi``
     - ``"SU2_CFD"`` / ``"mpirun"``
     - SU2 の実行ファイル名（またはパス）と MPI の起動コマンド（``ranks_per_case`` が 2 以上のときだけ使う）
   * - ``ranks_per_case``
     - ``4``
     - 1 ケースの MPI ランク数。0 で自動（4 とスレッド数の小さいほう）
   * - ``parallel_cases``
     - ``4``
     - 同時に解くケース数。0 で自動（メモリとスレッドの予算に収まる数）。``ranks_per_case`` × ``parallel_cases`` が使えるスレッド数を超えると警告する
   * - ``timeout_minutes``
     - ``0``
     - 1 ケースの時間の上限 [分]。0 で無制限。上限に達したケースは、係数が落ち着いていれば受け入れ、そうでなければ失敗とする
   * - ``workdir``
     - ``"cfd"``
     - ``output.dir`` の下の作業ディレクトリ（ケースごとのサブディレクトリとメッシュのキャッシュ）

CFD の準備は ``ignisyeet cfd-check 設定ファイル [--mesh]`` で確かめる。結果は ``cfd_cases.csv`` と ``cfd_report.json`` に書かれる（後述）。

``[earth]``

.. list-table::
   :header-rows: 1
   :widths: 27 22 51

   * - 項目
     - 既定値
     - 意味
   * - ``model``
     - ``"flat"``
     - 積分する座標系。``"flat"`` は射点の局所 ENU 座標系（平面地球・慣性系）、
       ``"ecef"`` は WGS84 楕円体に固定した回転座標系（Coriolis 力と遠心力を含む。:ref:`sec-ecef`）
   * - ``gravity``
     - ``"inverse_square"``
     - 重力モデル。``"constant"``\ （:math:`g_0` 一定）、``"inverse_square"``\ （逆 2 乗則）、
       ``"j2"``\ （点質量 + :math:`J_2` 項。:ref:`sec-gravity`）。``"j2"`` は ``model = "ecef"`` のときだけ指定できる

``[atmosphere]``

.. list-table::
   :header-rows: 1
   :widths: 27 22 51

   * - 項目
     - 既定値
     - 意味
   * - ``model``
     - ``"us1976"``
     - ``"us1976"``\ （米国標準大気 1976）または ``"constant"``\ （高度によらず一定）
   * - ``temperature_offset``
     - ``0.0``
     - 標準大気の温度に一律に足す温度差 :math:`\Delta T` [K]（``us1976`` のみ。:math:`-200` K より大きい値）
   * - ``density``
     - ``1.225``
     - 密度 [kg/m\ :sup:`3`]（``constant`` のみ）
   * - ``sound_speed``
     - ``340.29``
     - 音速 [m/s]（``constant`` のみ）
   * - ``viscosity``
     - ``1.789e-5``
     - 粘性係数 [Pa s]（``constant`` のみ）

``constant`` では三つの値がいずれも正でなければならない。この設定は飛翔計算だけでなく、
摩擦抗力の Reynolds 数（射点高度の大気）にも使われ、係数表の入力ハッシュにも含まれる。

``[wind]``

.. list-table::
   :header-rows: 1
   :widths: 27 22 51

   * - 項目
     - 既定値
     - 意味
   * - ``model``
     - ``"power"``
     - 風速の高度分布。``"constant"``\ （一定風）、``"power"``\ （べき法則）、``"log"``\ （対数則）、
       ``"profile"``\ （高度別の表。:ref:`sec-wind`）
   * - ``speed``
     - ``4.0``
     - 基準高度での風速 [m/s]（0 以上）。``profile`` では使わず、表から求めた基準高度の値で置き換える
   * - ``direction_deg``
     - ``0.0``
     - 風が **吹いてくる** 方位（北から時計回り）[deg]。``profile`` では使わない
   * - ``ref_height``
     - ``2.0``
     - 風速の基準高度 [m]（正）
   * - ``exponent``
     - ``6.0``
     - べき法則の指数 :math:`n`\ （正。``power`` のみ）
   * - ``roughness_length``
     - ``0.03``
     - 粗度長 :math:`z_0` [m]（``log`` のみ。0 より大きく ``ref_height`` より小さい値）
   * - ``ground_blend_height``
     - ``1.0``
     - 地面付近で速度を 0 になめらかにつなぐ高さ [m]（``power`` と ``log`` のみ。正で ``ref_height`` より小さい値）。``log`` では :math:`z_0` より大きい必要があり、設定値が :math:`z_0` 以下なら :math:`\max(\text{値}, 2 z_0)` に読み替える。地面での無限大・不連続な勾配を取り除き、適応刻み幅の積分が着地付近で極端に小さくならないようにする
   * - ``profile``
     - なし
     - ``model = "profile"`` のとき必須。高度別の風の CSV ファイル（形式は :ref:`sec-wind-profile`）

``[sim]``

.. list-table::
   :header-rows: 1
   :widths: 27 22 51

   * - 項目
     - 既定値
     - 意味
   * - ``integrator``
     - ``"rk4"``
     - 数値積分法。``"rk4"``\ （固定刻みの古典的 Runge–Kutta 法）、``"rk45"``\ （刻み幅制御付きの Dormand–Prince 5(4) 法）、
       ``"dop853"``\ （刻み幅制御付きの Dormand–Prince 8(5,3) 法）のいずれか（:ref:`sec-integrators`）
   * - ``attitude``
     - ``"normalize"``
     - 姿勢の更新法。``"normalize"`` は四元数を通常の状態量として積分し、各ステップ後に正規化する。
       ``"lie_group"`` は SO(3) 上の Runge–Kutta–Munthe-Kaas 法で、正規化をせずに四元数のノルムが丸め誤差の範囲で 1 に保たれる。
       どの積分法とも組み合わせられる（:ref:`sec-integrators`）
   * - ``dt``
     - ``0.002``
     - ``rk4`` の時間刻み [s]。``rk45``・``dop853`` では最初の刻み幅として使う
   * - ``rtol`` / ``atol``
     - ``1e-7`` / ``1e-6``
     - ``rk45``・``dop853`` の相対許容誤差・絶対許容誤差（いずれも正）
   * - ``max_time``
     - ``1200``
     - 計算を打ち切る時刻 [s]
   * - ``output_interval``
     - ``0.05``
     - ``trajectory.csv`` に書き出す間隔 [s]。``rk45``・``dop853`` では刻み幅の上限でもある（正）
   * - ``descent``
     - 自動
     - ``sim`` の降下モード。省略時はパラシュートが有効なら ``"parachute"``、無効なら ``"ballistic"``

``[dispersion]``

.. list-table::
   :header-rows: 1
   :widths: 27 22 51

   * - 項目
     - 既定値
     - 意味
   * - ``mode``
     - ``"wind_grid"``
     - ``"wind_grid"``\ （風速 × 風向の格子）または ``"monte_carlo"``\ （:doc:`dispersion`）
   * - ``wind_speeds``
     - ``[1, 2, …, 7]``
     - ``wind_grid`` で計算する基準風速の一覧 [m/s]
   * - ``directions``
     - ``8``
     - ``wind_grid`` の風向の分割数（北から等間隔。1 以上）

``[dispersion.monte_carlo]``\ （``mode = "monte_carlo"`` のときに使う）

各ばらつきは互いに独立な正規分布 :math:`N(0,\sigma^2)` から引き、以下の値は標準偏差 :math:`\sigma` である。
0 を指定するとそのばらつきを無効にする。負の値や有限でない値はエラーになる。
「相対」は公称値に対する割合（0.03 なら 3 %。公称値に :math:`1+\text{乱数}` を掛け、下限は 0.05）、
「絶対」は公称値に加える量（単位は項目ごとに示す）である。

.. list-table::
   :header-rows: 1
   :widths: 27 22 51
   :class: longtable

   * - 項目
     - 既定値
     - 意味
   * - ``samples``
     - ``1000``
     - サンプル数（1 以上）
   * - ``seed``
     - ``1``
     - 乱数の種。サンプル :math:`i` は ``seed + i`` を種とする
   * - ``thrust_scale``
     - ``0.03``
     - 相対。推力（全力積が比例して変わる。推進剤質量は変えない）
   * - ``burn_time_scale``
     - ``0.02``
     - 相対。燃焼時間の伸縮（推力曲線の時間軸を伸ばす）
   * - ``dry_mass``
     - ``0.1``
     - 相対。乾燥質量（慣性モーメントも同じ比で変わる）
   * - ``cg``
     - ``0.01``
     - 絶対 [m]。乾燥重心の位置（正が後方）
   * - ``cn_scale``
     - ``0.10``
     - 相対。法線力係数 :math:`C_N`、:math:`C_{N\alpha}`、ピッチ減衰
   * - ``ca_scale``
     - ``0.15``
     - 相対。軸力（抗力）係数 :math:`C_A`\ （燃焼中・燃焼後）
   * - ``elevation_deg``
     - ``0.5``
     - 絶対 [deg]。ランチャの射角
   * - ``azimuth_deg``
     - ``1.0``
     - 絶対 [deg]。ランチャの射方位
   * - ``wind_speed``
     - ``1.0``
     - 絶対 [m/s]。基準風速（``[wind]`` の ``speed`` の周りに分布し、0 未満は 0 にする）
   * - ``wind_direction_deg``
     - ``15.0``
     - 絶対 [deg]。風向（``[wind]`` の ``direction_deg`` の周りに分布する）
   * - ``parachute_cd_s_scale``
     - ``0.1``
     - 相対。パラシュートの抗力面積 :math:`C_DS`

各項目の扱いの詳細は :ref:`sec-mc` で述べる。

.. _sec-aero-table-format:

外部の係数表（``aero.method = "table"``）
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

風洞試験や CFD などで得た係数を使うときは、``aero.method = "table"`` と ``aero.table = "cfd_table.csv"`` を指定する。
STL からの形状抽出と空力モデルの計算は行わず、``sim`` と ``dispersion`` はこの表をそのまま使う（``aero`` は表の行数を表示するだけである）。
形式は ``out/aero_table.csv`` と同じで、CSV と、拡張子を ``.json`` に替えた同名の付随ファイル（メタデータ）の 2 つが必要である。

CSV の 1 行目は次の見出しで、続く行は Mach 数が外側、迎角が内側の順（Mach 数ごとに迎角を小さい方から並べる）に、全格子点を 1 行ずつ書く。

.. code-block:: text

   mach,alpha_deg,cn,ca_on,ca_off,xcp,cna,damp_s0,damp_s1,damp_s2

各列は、法線力係数 :math:`C_N`、軸力係数（燃焼中・燃焼後）、圧力中心 :math:`x_{cp}` [m]（ノーズ先端から）、
:math:`C_{N\alpha}` [1/rad]、ピッチ減衰の和 :math:`S_0, S_1, S_2`\ （:eq:`eq-moment-damp`）である。

付随の ``.json`` には、基準面積 ``ref_area`` [m\ :sup:`2`]、基準直径 ``ref_diameter`` [m]、全長 ``length`` [m]、
格子 ``machs`` と ``alphas_deg``\ （各列の値）、``extrapolation``\ （``"linear"`` または ``"clamp"``）、``source_hash``\ （任意の文字列でよい）を書く。
行数が ``machs`` と ``alphas_deg`` の積に一致しないとエラーになる。``motor.aft_x`` を省略したときのモータ後端位置は、この ``length`` になる。

設定ファイルの未知の項目は、綴りの誤りを見逃さないようにすべてエラーとして扱う。

出力ファイル
------------

.. list-table::
   :header-rows: 1
   :widths: 30 70
   :class: longtable

   * - ファイル
     - 内容
   * - ``geometry.json``
     - 抽出した形状（全長、基準直径、ノーズ、ボートテール、フィン、断面ごとの半径など）
   * - ``profile.csv``
     - 断面ごとの胴体半径 ``r`` とフィン高さ ``fin_span``
   * - ``aero_table.csv`` / ``.json``
     - (Mach, 迎角) ごとの :math:`C_N`, :math:`C_A`\ （燃焼中・燃焼後）, :math:`x_{cp}`, :math:`C_{N\alpha}`、ピッチ減衰の和。``.json`` は格子・基準量・入力ハッシュ
   * - ``aero_drag.csv``
     - Mach 数ごとの抗力の内訳、:math:`C_{N\alpha}`、:math:`x_{cp}`\ （``method = "panel"`` では内訳なしで合計の ``cd_off``, ``cd_on`` のみ）
   * - ``cfd_cases.csv``
     - ``method = "cfd"``。ケースごとの状態（``converged``, ``accepted``, ``failed``）、反復数、残差、SU2 の生の係数、使う係数、計算時間、暖機起動の有無
   * - ``cfd_report.json``
     - ``method = "cfd"``。ケース数、失敗と未収束の一覧、メッシュの節点数とセル数、総計算時間、測定した :math:`\alpha=0` の非対称性、実行の並列度と 1 ケースのメモリの見積もり
   * - ``panel_cp.csv``
     - ``method = "panel"``。表面パネルの位置・法線・面積・圧力係数 ``cp``\ （迎角 4°、最も低い亜音速 Mach 数）と部位（``body`` または ``fin``）。
       ``python/plot.py panel`` で作図する
   * - ``trajectory.csv``
     - 時刻歴（位置、緯度経度、速度、Mach 数、迎角、推力、質量、重心、圧力中心、安定余裕、動圧、姿勢、角速度）
   * - ``summary.json``
     - ランチャ離脱速度、最大 Mach 数、最大動圧、頂点、最小安定余裕、着地点など
   * - ``ignisyeet.kml``
     - ``kml = true`` のとき ``sim`` と ``dispersion`` が書き出す、Google Earth 用の KML ファイル 1 つ。射点、飛翔経路（段階ごとに色分け）とイベントの印、風の格子の着地点の輪郭、モンテカルロの着地点と誤差楕円をフォルダに分けて収める（:ref:`sec-kml`）
   * - ``dispersion.csv``
     - ``mode = "wind_grid"``。風の各ケースの着地点（ENU と緯度経度）、頂点、飛行時間など
   * - ``dispersion_mc.csv``
     - ``mode = "monte_carlo"``。サンプルごと・降下モードごとの 1 行。サンプル番号、降下モード、``status``\ （``ok`` または ``failed``）、
       引いたばらつき（相対のものは割合、``wind_speed_delta`` と ``wind_direction_deg_delta`` は風速・風向のずれ）、
       そのサンプルの基準風速・風向、着地点（ENU と緯度経度）、着地距離、着地時刻、着地速度、頂点、頂点時刻、最大 Mach 数、
       ランチャ離脱速度、最小静安定余裕。失敗した行は ``status`` 以降の結果が空欄になる
   * - ``dispersion_summary.json``
     - ``mode = "monte_carlo"``。降下モードごとの有効サンプル数 ``n``、失敗数 ``failed``、着地点の平均・共分散行列、
       :math:`1\sigma` と :math:`3\sigma` の誤差楕円（長半径・短半径・長軸の方位）、最大着地距離とそのサンプル番号、頂点高度の平均と標準偏差

.. _sec-kml:

Google Earth で見る（KML）
--------------------------

``[output]`` の ``kml`` が ``true``\ （既定）のとき、``sim`` と ``dispersion`` は ``ignisyeet.kml`` を、CSV や JSON と同じ出力先ディレクトリに書き出す。
KML は地理情報を記述する XML 形式で、OGC の標準（KML 2.2 :cite:`ogckml`）である。Google Earth（デスクトップ版・Web 版）や QGIS などで開くと、機体の経路や着地点を衛星写真の上に重ねて確認できる。
不要なときは ``kml = false`` にすればよい。各要素をクリックすると、時刻・速度・着地点などの数値が説明欄に表示される（説明欄の文は英語である）。

``ignisyeet.kml`` は 1 つの KML 文書で、次のフォルダ（Google Earth の左のパネルでチェックボックスにより表示を切り替えられる）からなる。
``sim`` と ``dispersion`` は別のコマンドなので、どちらも実行のたびにこのファイルを書き直す。
そのコマンドで計算した結果に加え、同じ出力先ディレクトリにある他の結果（``trajectory.csv`` と ``summary.json``、``dispersion.csv``、``dispersion_mc.csv`` と ``dispersion_summary.json``）を読み戻して、1 つにまとめる。
該当するファイルがなければ、そのフォルダは作らない（読めないファイルがあるときは、そのフォルダを省いたことを端末に表示する）。
したがって、``sim`` のあとに ``dispersion`` を実行すれば、飛翔経路と落下分散を同じ画面で重ねて見られる。
読み戻す結果は、以前に同じディレクトリへ書いたものである。設定を変えたときは、``sim`` と ``dispersion`` の両方を実行し直すこと。

``Launch site``
   射点を示す白いピンで、地面に固定して表示する（射角・方位・レール長を説明欄に載せる）。

``Flight``
   1 回の飛翔（``sim`` で選んだ降下モード）の内容で、``sim`` の結果（``trajectory.csv`` と ``summary.json``）があるときに入る。

   * **飛翔経路**：段階ごとに別の線にして色分けする。黄がランチャ滑走、赤が燃焼中、青が燃焼後の慣性飛行、緑がパラシュート降下である。
     線だけを描き、地面への塗りつぶしはしない。
   * **イベントの印**：ランチャ離脱（Rail exit。そのときの静安定余裕を含む）、燃焼終了（Burnout）、頂点（Apogee）、開傘（Parachute deploy。パラシュート降下のときだけ）、着地（Landing）。
     各印の説明欄に時刻・高度・速度などを載せる。着地の印は地面に固定し、射点からの距離と東・北方向の距離、着地速度を載せる。
   * フォルダの説明欄に、風の条件と色の凡例を載せる。

``Dispersion – wind grid``
   風の格子の結果（``dispersion.csv``）があるときに入る。降下モード（``ballistic`` と ``parachute``）ごとのフォルダの下に、風速ごとのフォルダがある。
   風速ごとに、同じ風速で風向を変えた着地点を風向の順に結んだ閉じた折れ線（輪郭）と、各ケースの着地点の印（風速と風向を名前にする）が入る。
   弾道落下は青、パラシュート降下は橙で、風速が大きいほど濃い色にする。

``Dispersion – Monte Carlo``
   モンテカルロの結果（``dispersion_mc.csv`` と ``dispersion_summary.json``）があるときに入る。降下モードごとのフォルダに、平均着地点（Mean landing）、:math:`1\sigma` と :math:`3\sigma` の誤差楕円（:eq:`eq-mc-stat` の共分散から作る 72 頂点の閉じた折れ線で、塗りつぶさない）、
   各サンプルの着地点（``Landing points`` フォルダ）が入る。失敗したサンプルは含まない。

落下分散の図形はすべて地面に固定して描く。

**高度の扱い。** 飛翔経路の高度は ``absolute``\ （絶対高度）で書き出す。値は WGS84 楕円体からの高さで、射点で ``launch.altitude`` から始まり、
飛翔中は ``launch.altitude`` に射点からの上向きの高さを足したものになる（ECEF モードでは楕円体高そのもの）。
KML の絶対高度は名目上は平均海面（EGM96 ジオイド）からの高度だが、IgnisYeet はジオイド高（楕円体と平均海面の差）を **扱わない**。
場所によっては数十 m の差になるので、``launch.altitude`` には射点の **海抜標高** を入れ、地形の高さと合っていることを確かめてほしい。
合っていなければ、経路が地面にめり込んだり浮いたりして見える。射点・着地点・誤差楕円など地面に固定する図形は、Google Earth の地形に沿って表示される。

ディレクトリ構成
----------------

リポジトリの主なファイルとディレクトリを :numref:`fig-directory-tree` に示す。

.. _fig-directory-tree:

.. figure:: _generated/tikz/directory_tree.*
   :width: 100%

   リポジトリの構成。フォルダのアイコンはディレクトリ、紙のアイコンはファイルを表す。

.. _sec-doc-build:

ドキュメントのビルド
--------------------

このドキュメント（Sphinx）は ``doc/`` にある。図は TikZ（LuaLaTeX + dvisvgm）と matplotlib で作り、
matplotlib の図に使うデータは本体を実行して作る（``make data``。最初に ``cargo build --release`` も走る）。
``make html`` と ``make pdf`` はどちらも、これらの図とデータを作り直してから文書を生成する。

.. code-block:: sh

   cd doc
   make html             # HTML を生成する。build/html/index.html を開く
   make pdf              # PDF を生成する。doc/build/latex/ignisyeet.pdf ができる

Python の環境は uv でも pip でも用意できる。Makefile は既定では ``uv run --project .`` 経由で
``sphinx-build`` と ``python`` を呼ぶ（初回に依存パッケージが自動で入る）。

.. code-block:: sh

   # uv の場合（そのまま実行する）
   make html

   # pip の場合: 仮想環境に入れて、PY を空にすると有効な環境の sphinx-build / python をそのまま呼ぶ
   python3 -m venv .venv && . .venv/bin/activate
   pip install -r doc/requirements.txt
   make html PY=
   make pdf  PY=

``doc/requirements.txt`` には Sphinx、furo、sphinxcontrib-bibtex と、図の作成に使う numpy、pandas、matplotlib が入っている。

必要なソフトウェアは次のとおりである。

.. list-table::
   :header-rows: 1
   :widths: 30 70

   * - ソフトウェア
     - 用途
   * - Rust（``cargo``）
     - 図のデータを作る本体のビルドと実行
   * - LuaLaTeX（luatexja）と dvisvgm
     - TikZ 図（HTML 用の SVG と PDF 用の PDF）
   * - inkscape
     - ロゴ（SVG）を PDF に変換する（PDF 版のみ）
   * - upLaTeX、dvipdfmx、latexmk
     - PDF 版の組版。TeX Live なら ``texlive-lang-japanese`` など日本語関連のパッケージを含めて入れる
   * - IPAex フォント
     - 図と PDF 本文の日本語

PDF の組版中に LaTeX がエラーで止まったときは、``doc/build/latex/ignisyeet.log`` を開き、``!`` で始まる行とその直後の行番号を確認する。
多くはパッケージ不足かフォント不足である。原因を直したあと、もう一度 ``make pdf`` を実行すればよい。
