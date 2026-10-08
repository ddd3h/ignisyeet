.. _errors-install:

インストーラのエラーとワーニング
================================

``install.sh`` と ``scripts/*.sh`` が出すメッセージの一覧である。
``error[IY-I010]:`` や ``warning[IY-I510]:`` のように、メッセージの先頭にコードが付く。
番号は 001〜499 がエラー、500〜999 がワーニングである。
エラーでは、インストーラは終了コード 1 で停止する。ワーニングでは処理を続ける。

.. _IY-I001:

IY-I001
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I001]: unknown option: <option> (see --help)

意味
   インストーラが知らないオプションが指定された。

主な原因
   オプションの綴り違い。存在しないオプション名を使った。

対処
   ``bash install.sh --help`` で指定できるオプションを確認する。

.. _IY-I002:

IY-I002
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I002]: <option> needs a value

意味
   ``--version``、``--prefix``、``--dir`` のいずれかに値が付いていない。

主な原因
   オプションの直後に値がない。シェルの変数展開が空になった。

対処
   値を付けて再実行する（例：``--version v0.4.0``、``--prefix ~/.local``）。

.. _IY-I003:

IY-I003
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I003]: version must match vX.Y.Z (got '<value>')

意味
   ``--version`` または ``IGNISYEET_VERSION`` の形式が ``vX.Y.Z`` ではない。

主な原因
   先頭の ``v`` がない。``v0.4`` のように桁が足りない。

対処
   ``v0.4.0`` の形式で指定する。

.. _IY-I004:

IY-I004
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I004]: role must be 'user' or 'developer' (got '<value>')

意味
   ``IGNISYEET_ROLE`` が ``user`` でも ``developer`` でもない。

主な原因
   環境変数の値の綴り違い。

対処
   ``user`` または ``developer`` にするか、変数を unset する。

.. _IY-I005:

IY-I005
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I005]: IGNISYEET_CFD must be 1 or 0

意味
   ``IGNISYEET_CFD`` が 1 でも 0 でもない。

主な原因
   ``yes`` や ``true`` など別の表記を使った。

対処
   ``1``\ （CFD を入れる）か ``0``\ （入れない）にする。

.. _IY-I006:

IY-I006
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I006]: --prefix must not be empty

意味
   インストール先 ``--prefix`` が空文字列である。

主な原因
   ``--prefix "$VAR"`` の変数が空だった。

対処
   空でないディレクトリを指定する。

.. _IY-I010:

IY-I010
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I010]: unsupported platform: <OS>/<ARCH> (supported: Linux x86_64/aarch64, macOS Intel/Apple Silicon)

意味
   この OS と CPU の組み合わせには、配布バイナリがない。

主な原因
   Windows、BSD、32 ビット環境、その他のアーキテクチャで実行した。

対処
   対応環境で実行する。それ以外では ``--developer`` でソースからビルドできるか、Rust の対応状況を確認する。

.. _IY-I011:

IY-I011
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I011]: cannot create <data directory>

意味
   データディレクトリ（既定は ``~/.local/share/ignisyeet``）を作れない。

主な原因
   書き込み権限がない。ディスクが満杯である。親のパスがファイルである。

対処
   権限と空き容量を確認するか、``--prefix`` で書き込めるディレクトリを指定する。

.. _IY-I012:

IY-I012
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I012]: cannot write <log file>

意味
   インストールログを書き込めない。

主な原因
   データディレクトリの権限不足。ディスクが満杯である。

対処
   権限と空き容量を確認する。

.. _IY-I020:

IY-I020
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I020]: IGNISYEET_DOWNLOAD_BASE is set; also give --version vX.Y.Z

意味
   ダウンロード元を ``IGNISYEET_DOWNLOAD_BASE`` で差し替えたのに、バージョンが決まっていない。最新版の問い合わせは GitHub に対してしか行えない。

主な原因
   ミラーやローカルのファイルサーバを指定して ``--version`` を省いた。

対処
   ``--version vX.Y.Z`` を一緒に指定する。

.. _IY-I021:

IY-I021
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I021]: could not determine the latest release of <repo> (no release published yet, or no network). Use --version vX.Y.Z or --developer.

意味
   最新リリースのバージョンを GitHub から取得できない。

主な原因
   リリースがまだ公開されていない。ネットワークに接続できない。プロキシや API の制限。

対処
   ``--version vX.Y.Z`` で明示するか、``--developer`` でソースから入れる。ネットワークを確認する。

.. _IY-I022:

IY-I022
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I022]: Python >= 3.10 is required for plotting; install python3 (>= 3.10) or allow uv to be installed.

意味
   作図用の Python 環境に必要な Python 3.10 以上が見つからない。

主な原因
   python3 がない、または 3.9 以下である。uv のインストールを断った。

対処
   python3 (3.10 以上) を入れるか、uv のインストールを許可する（uv が Python を用意する）。

.. _IY-I030:

IY-I030
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I030]: uv installation failed

意味
   作図環境の構築に使う uv をインストールできなかった。

主な原因
   ネットワークに接続できない。``astral.sh`` にアクセスできない。

対処
   ログ（``log:`` に表示されるファイル）を見て原因を確認する。uv を手で入れて再実行する。

.. _IY-I031:

IY-I031
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I031]: uv was installed but cannot be found

意味
   uv のインストールは終わったが、実行ファイルが見つからない。

主な原因
   uv が通常と違う場所に入った。

対処
   uv の場所を ``PATH`` に加えて再実行する。

.. _IY-I032:

IY-I032
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I032]: uv sync failed

意味
   ロックされた作図環境（または開発用の ``python/``、``doc/``）を ``uv sync`` で作れなかった。メッセージは ``uv sync python failed``、``uv sync doc failed`` の形でも出る。

主な原因
   ネットワークの不通。パッケージの取得失敗。ディスクの空き不足。

対処
   ログの末尾を確認する。解消後に再実行する。

.. _IY-I033:

IY-I033
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I033]: uv venv failed

意味
   uv で仮想環境を作れなかった。

主な原因
   書き込み権限がない。要求する Python が見つからず取得もできない。

対処
   ログを確認する。ディスクと権限を調べる。

.. _IY-I034:

IY-I034
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I034]: uv pip install failed

意味
   uv で numpy、pandas、matplotlib を入れられなかった。

主な原因
   ネットワークの不通。PyPI に接続できない。

対処
   ネットワークを確認して再実行する。

.. _IY-I035:

IY-I035
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I035]: python3 -m venv failed (on Debian/Ubuntu: apt install python3-venv)

意味
   ``python3 -m venv`` で仮想環境を作れなかった。

主な原因
   Debian 系で ``python3-venv`` が入っていない。

対処
   ``sudo apt install python3-venv`` を実行して再実行する。

.. _IY-I036:

IY-I036
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I036]: pip install failed

意味
   pip で作図用のパッケージを入れられなかった。

主な原因
   ネットワークの不通。制約ファイルと合うバージョンが取得できない。

対処
   ログを確認して再実行する。

.. _IY-I040:

IY-I040
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I040]: curl and tar are required

意味
   ダウンロードと展開に必要な ``curl`` と ``tar`` のどちらかがない。

主な原因
   最小構成のコンテナなど。

対処
   ``curl`` と ``tar`` を入れる。

.. _IY-I041:

IY-I041
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I041]: sha256sum or shasum is required to verify the download

意味
   ダウンロードの検証に使う ``sha256sum`` か ``shasum`` がない。

主な原因
   最小構成の環境。

対処
   どちらかを入れる。検証を省いて入れることはできない。

.. _IY-I042:

IY-I042
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I042]: download failed: <url> (does release <version> exist for <target>?)

意味
   リリースアーカイブをダウンロードできなかった。

主な原因
   指定したバージョンが存在しない。そのターゲット向けのアーカイブがない。ネットワークの不通。

対処
   ``--version`` を確認する。URL をブラウザで開いて存在を確かめる。ログを見る。

.. _IY-I043:

IY-I043
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I043]: checksum download failed

意味
   アーカイブの ``.sha256`` ファイルをダウンロードできなかった。

主な原因
   リリースにチェックサムが添付されていない。ネットワークの不通。

対処
   再実行する。続く場合はリリースの添付ファイルを確認する。

.. _IY-I044:

IY-I044
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I044]: sha256 mismatch for <archive> (expected <sha256>, got <sha256>); refusing to install

意味
   ダウンロードしたアーカイブのチェックサムが公開値と一致しない。インストールは行わない。

主な原因
   ダウンロードが途中で壊れた。プロキシやミラーが内容を書き換えた。リリースの差し替え。

対処
   再実行して再度ダウンロードする。繰り返す場合はそのネットワーク経路を信用せず、別の経路で取得する。

.. _IY-I045:

IY-I045
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I045]: extraction failed

意味
   アーカイブを展開できなかった。

主な原因
   ディスクの空き不足。ダウンロードの破損。

対処
   空き容量を確認して再実行する。

.. _IY-I046:

IY-I046
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I046]: archive has an unexpected layout (no <name>/bin/ignisyeet)

意味
   アーカイブの中に ``bin/ignisyeet`` がない。

主な原因
   リリースの構成が想定と異なる。インストーラが古い。

対処
   最新のインストーラを取得して再実行する。

.. _IY-I047:

IY-I047
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I047]: installation failed

意味
   展開したファイルを所定の場所に配置できなかった。

主な原因
   書き込み権限がない。ディスクが満杯である。

対処
   権限と空き容量を確認する。ログの末尾を見る。

.. _IY-I050:

IY-I050
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I050]: rustup installation failed

意味
   開発者向けインストールで必要な Rust（rustup）を入れられなかった。

主な原因
   ネットワークの不通。

対処
   ネットワークを確認する。https://rustup.rs の手順で手動インストールして再実行する。

.. _IY-I051:

IY-I051
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I051]: cargo not found after installing Rust

意味
   Rust を入れたが ``cargo`` が見つからない。

主な原因
   ``~/.cargo/bin`` が ``PATH`` にない。

対処
   ``source ~/.cargo/env`` を実行して再実行する。

.. _IY-I052:

IY-I052
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I052]: git is required for the developer install (apt install git / xcode-select --install)

意味
   開発者向けインストールに必要な ``git`` がない。

主な原因
   git が入っていない。

対処
   git を入れる（Debian 系は ``apt install git``、macOS は ``xcode-select --install``）。

.. _IY-I053:

IY-I053
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I053]: <dir> exists and is not an IgnisYeet clone

意味
   クローン先に、IgnisYeet のクローンではない空でないディレクトリがある。

主な原因
   ``--dir`` が別の用途のディレクトリを指している。

対処
   別の ``--dir`` を指定するか、そのディレクトリを片付ける。

.. _IY-I054:

IY-I054
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I054]: git clone failed

意味
   リポジトリを clone できなかった。

主な原因
   ネットワークの不通。``IGNISYEET_REPO_URL`` の誤り。

対処
   ネットワークと URL を確認する。ログを見る。

.. _IY-I055:

IY-I055
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I055]: cannot check out <version>

意味
   指定したタグをチェックアウトできない。

主な原因
   タグが存在しない。ローカルの変更と衝突している。

対処
   ``--version`` を確認する。作業ツリーの変更を整理する。

.. _IY-I056:

IY-I056
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I056]: build failed

意味
   ``cargo build --release`` が失敗した。

主な原因
   Rust のバージョンが古い。依存の取得失敗。リンカやコンパイラがない。

対処
   ログの末尾のエラーを確認する。``rustup update`` を試す。

.. _IY-I057:

IY-I057
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I057]: tests failed (see the log)

意味
   ``cargo test --release`` が失敗した。

主な原因
   環境固有の不具合。変更途中のソース。

対処
   ログで失敗したテストを確認する。テストを省くなら ``--no-test`` を付ける。

.. _IY-I060:

IY-I060
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I060]: binary not found: <path>

意味
   リリースのパッケージ作成スクリプト ``scripts/package-release.sh`` に渡したバイナリがない。

主な原因
   ビルドしていない。パスの誤り。

対処
   ``cargo build --release`` の後に正しいパスを渡す。

.. _IY-I061:

IY-I061
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I061]: GNU tar is required

意味
   パッケージ作成スクリプトに GNU tar が必要である。

主な原因
   macOS の BSD tar を使っている。

対処
   GNU tar（``gtar``）を入れる。

.. _IY-I090:

IY-I090
-------

重大度
   エラー

メッセージ
   .. code-block:: text

      error[IY-I090]: unexpected error (line <n>, status <rc>)

意味
   インストーラの想定外の失敗である。個別のエラーコードがない箇所で、コマンドが失敗した。

主な原因
   環境固有の問題。インストーラの不具合。

対処
   ``details:`` に表示されるログを見る。再現する場合は、ログを添えて報告する。

.. _IY-I501:

IY-I501
-------

重大度
   ワーニング

メッセージ
   .. code-block:: text

      warning[IY-I501]: interrupted

意味
   利用者が ``Ctrl-C`` で中断した。終了コードは 130 である。

主な原因
   利用者の操作。

対処
   必要なら再実行する。途中まで進んだ部分は、再実行時に再利用される。

.. _IY-I510:

IY-I510
-------

重大度
   ワーニング

メッセージ
   .. code-block:: text

      warning[IY-I510]: su2 <version> needs AVX-512, which this CPU lacks (SU2_CFD will crash); recreate the env with su2=8.3.0

意味
   CFD 環境の SU2 が 8.5.0 系で、AVX-512 を持たないこの CPU では異常終了する。後半の文言が ``recreate with: …`` のコマンド付きになる場合もある。

主な原因
   古い環境を作ったときに SU2 のバージョンを固定していなかった。

対処
   環境を作り直す。``--cfd --force`` で再実行すると 8.3.0 の環境になる。:ref:`sec-cfd-native` も参照。

.. _IY-I520:

IY-I520
-------

重大度
   ワーニング

メッセージ
   .. code-block:: text

      warning[IY-I520]: working tree has local changes; not updating it

意味
   開発者向けインストールで、既存のクローンに未コミットの変更があるため更新しない。

主な原因
   クローンで作業中である。

対処
   変更をコミットまたは退避してから ``git pull`` する。

.. _IY-I521:

IY-I521
-------

重大度
   ワーニング

メッセージ
   .. code-block:: text

      warning[IY-I521]: git pull failed; continuing with the current checkout

意味
   ``git pull --ff-only`` が失敗した。現在のチェックアウトのまま進める。

主な原因
   fast-forward できない。ネットワークの不通。

対処
   必要なら手動で ``git pull`` する。

.. _IY-I522:

IY-I522
-------

重大度
   ワーニング

メッセージ
   .. code-block:: text

      warning[IY-I522]: doc requirements failed

意味
   ドキュメントのビルド用 Python パッケージを入れられなかった。他の部分は続行する。

主な原因
   ネットワークの不通。

対処
   ログを確認し、``pip install -r doc/requirements.txt`` を手動で実行する。

.. _IY-I523:

IY-I523
-------

重大度
   ワーニング

メッセージ
   .. code-block:: text

      warning[IY-I523]: missing: <tools>

意味
   ドキュメントを PDF までビルドするためのツール（TeX Live など）が足りない。このインストーラはこれらを自動では入れない。

主な原因
   TeX Live、dvisvgm、mutool、inkscape、IPAex フォントのいずれかがない。

対処
   直後に表示される ``apt`` または ``brew`` のコマンドで入れる。ドキュメントを作らないなら無視してよい。

.. _IY-I530:

IY-I530
-------

重大度
   ワーニング

メッセージ
   .. code-block:: text

      warning[IY-I530]: micromamba installation failed; CFD skipped

意味
   CFD 環境を作る micromamba を入れられなかったため、CFD のインストールを省いた。

主な原因
   ネットワークの不通。そのプラットフォーム向けのバイナリがない。

対処
   ネットワークを確認して ``--cfd`` 付きで再実行する。conda や mamba を先に入れてもよい。

.. _IY-I531:

IY-I531
-------

重大度
   ワーニング

メッセージ
   .. code-block:: text

      warning[IY-I531]: CFD environment could not be created (SU2 may be unavailable for <platform>); CFD skipped

意味
   CFD 用の conda 環境を作れなかったため、CFD を省いた。

主な原因
   ネットワークの不通。ディスクの空き不足。そのプラットフォームに SU2 のパッケージがない。

対処
   ログを確認して再実行する。:ref:`sec-cfd-native` も参照。

.. _IY-I532:

IY-I532
-------

重大度
   ワーニング

メッセージ
   .. code-block:: text

      warning[IY-I532]: environment created but SU2_CFD was not found in it

意味
   環境は作れたが、SU2_CFD が中にない。

主な原因
   パッケージの構成の変化。

対処
   ``IGNISYEET_CFD_PREFIX`` に別の SU2 を指定するか、環境を作り直す。

.. _IY-I533:

IY-I533
-------

重大度
   ワーニング

メッセージ
   .. code-block:: text

      warning[IY-I533]: manifold3d could not be installed; surface = "stl" will not be available (retry: <command>)

意味
   ``manifold3d`` を pip で入れられなかった。``surface = "stl"`` が使えなくなる。

主な原因
   ネットワークの不通。ビルドに必要なツールがない。

対処
   メッセージ中の ``retry:`` のコマンドを実行する。:ref:`sec-cfd-stl` も参照。

.. _IY-I540:

IY-I540
-------

重大度
   ワーニング

メッセージ
   .. code-block:: text

      warning[IY-I540]: SU2 could not be compiled: <reason>

意味
   SU2 のソースビルドに失敗した。続く 2 行（``the portable conda-forge SU2 … stays in use``、``details: …``）とともに表示される。conda-forge の SU2 をそのまま使い、残りの CFD ツールはインストールされる。

主な原因
   MPI コンパイララッパーがない。C++ コンパイラがない。SU2 のソースを取得できない。meson による設定または ninja によるコンパイルの失敗。ビルドした SU2_CFD が起動しない。

対処
   ``<reason>`` を確認し、ログ（``details:``）の末尾を見る。``bash install.sh --cfd --force`` で再試行する。ソースビルドが不要なら ``--su2-binary`` を付ける。:ref:`sec-cfd-native` を参照。

.. _IY-I541:

IY-I541
-------

重大度
   ワーニング

メッセージ
   .. code-block:: text

      warning[IY-I541]: SU2 was not compiled (<reason>); the portable conda-forge SU2 is used

意味
   完了時の要約に出る、I540 と同じ失敗の再掲である。

主な原因
   IY-I540 と同じ。

対処
   IY-I540 を参照。

.. _IY-I542:

IY-I542
-------

重大度
   ワーニング

メッセージ
   .. code-block:: text

      warning[IY-I542]: <bin directory> is not in your PATH. Add this line to your shell rc file:

意味
   ``ignisyeet`` を置いたディレクトリが ``PATH`` に入っていない。

主な原因
   シェルの設定ファイルに ``PATH`` の行がない。

対処
   表示される ``export PATH=…`` の行をシェルの rc ファイルに追加する。``--add-path`` を付けると自動で追記される。
