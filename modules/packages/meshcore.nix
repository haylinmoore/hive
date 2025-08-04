{
  lib,
  python3Packages,
  pycayennelpp,
}:

python3Packages.buildPythonPackage rec {
  pname = "meshcore";
  version = "2.0.4";
  pyproject = true;

  src = ../package_src/meshcore_py;

  nativeBuildInputs = with python3Packages; [
    hatchling
  ];

  propagatedBuildInputs = with python3Packages; [
    pyserial
    pyserial-asyncio
    bleak
    protobuf
    cryptography
    requests
    pycayennelpp
  ];

  doCheck = false;
  pythonImportsCheck = false;
  dontCheckRuntimeDeps = true;

  meta = with lib; {
    description = "Python library for MeshCore radio networks";
    homepage = "https://github.com/meshcore/python";
    license = licenses.gpl3;
    maintainers = with lib.maintainers; [ haylinmoore ];
  };
}
