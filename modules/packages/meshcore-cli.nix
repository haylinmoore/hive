{
  lib,
  python3,
  fetchPypi,
  pkgs,
}:

python3.pkgs.buildPythonApplication rec {
  pname = "meshcore-cli";
  version = "1.1.11";
  pyproject = true;

  src = fetchPypi {
    pname = "meshcore_cli";
    inherit version;
    sha256 = "0frqsmzl5nsv4v9ypy9j3c02dpd1ynws6gnpy20zhgz3wbjyicwi";
  };

  nativeBuildInputs = with python3.pkgs; [
    hatchling
  ];

  propagatedBuildInputs = with python3.pkgs; [
    meshcore
    click
    requests
    tabulate
    pyqrcode
    prompt-toolkit
  ];

  dontCheckRuntimeDeps = true;

  meta = with lib; {
    description = "Command line interface to meshcore companion radios";
    homepage = "https://github.com/meshcore/meshcore-cli";
    license = licenses.mit;
    maintainers = with maintainers; [ haylinmoore ];
    mainProgram = "meshcore";
  };
}
