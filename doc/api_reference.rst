APIリファレンス
==============

このセクションでは、IgnisYeetの主要なAPIについて説明します。

コアクラス
----------

RocketState
~~~~~~~~~~~

ロケットの状態を表現するクラスです。

.. doxygenclass:: ignis::RocketState
   :members:

Vector3D
~~~~~~~~

3次元ベクトル演算クラスです。

.. doxygenclass:: ignis::Vector3D
   :members:

物理モデル基底クラス
------------------

GravityModel
~~~~~~~~~~~~

重力モデルの基底クラスです。

.. doxygenclass:: ignis::GravityModel
   :members:

AtmosphereModel
~~~~~~~~~~~~~~~

大気モデルの基底クラスです。

.. doxygenclass:: ignis::AtmosphereModel
   :members:

AerodynamicModel
~~~~~~~~~~~~~~~~

空力学モデルの基底クラスです。

.. doxygenclass:: ignis::AerodynamicModel
   :members:

重力モデル実装
--------------

UniformGravity
~~~~~~~~~~~~~~

一様重力場モデルです。

.. doxygenclass:: ignis::UniformGravity
   :members:

AltitudeDependentGravity
~~~~~~~~~~~~~~~~~~~~~~~~

高度依存重力モデルです。

.. doxygenclass:: ignis::AltitudeDependentGravity
   :members:

RotatingEarthGravity
~~~~~~~~~~~~~~~~~~~~

回転地球重力モデルです。

.. doxygenclass:: ignis::RotatingEarthGravity
   :members:

大気モデル実装
--------------

ConstantDensityAtmosphere
~~~~~~~~~~~~~~~~~~~~~~~~~

定密度大気モデルです。

.. doxygenclass:: ignis::ConstantDensityAtmosphere
   :members:

ISAAtmosphere
~~~~~~~~~~~~~

ISA標準大気モデルです。

.. doxygenclass:: ignis::ISAAtmosphere
   :members:

DynamicAtmosphere
~~~~~~~~~~~~~~~~~

動的大気モデルです。

.. doxygenclass:: ignis::DynamicAtmosphere
   :members:

空力学モデル実装
----------------

BasicDragModel
~~~~~~~~~~~~~~

基本抗力モデルです。

.. doxygenclass:: ignis::BasicDragModel
   :members:

MachDependentModel
~~~~~~~~~~~~~~~~~~

マッハ数依存モデルです。

.. doxygenclass:: ignis::MachDependentModel
   :members:

CompressibleFlowModel
~~~~~~~~~~~~~~~~~~~~~

圧縮性流れモデルです。

.. doxygenclass:: ignis::CompressibleFlowModel
   :members:

設定システム
------------

Configuration
~~~~~~~~~~~~~

設定ファイル管理クラスです。

.. doxygenclass:: ignis::Configuration
   :members:

シミュレーション制御
------------------

SimulationEngine
~~~~~~~~~~~~~~~~

シミュレーション実行エンジンです。

.. doxygenclass:: ignis::SimulationEngine
   :members:

ユーティリティ関数
------------------

数学関数
~~~~~~~~

.. doxygenfunction:: ignis::calculateMachNumber

.. doxygenfunction:: ignis::calculateReynoldsNumber

物理定数
~~~~~~~~

.. doxygenvariable:: ignis::EARTH_RADIUS

.. doxygenvariable:: ignis::EARTH_ROTATION_RATE

.. doxygenvariable:: ignis::STANDARD_GRAVITY
