window.showAccountHint = ({ name, detail }) => {
  document.getElementById('name').textContent = name;
  document.getElementById('detail').textContent = detail;
  const hint = document.getElementById('hint');
  hint.classList.remove('play');
  void hint.offsetWidth;
  hint.classList.add('play');
};
