import "cleave.js/dist/addons/cleave-phone.us";

const onLoad = () => {
  // Text prefix example
  const config1 = {
    prefix: "Prefix",
    delimiter: "-",
    blocks: [6, 4, 4, 7],
  };

  const inputmask1 = document.querySelector("#inputmask1");
  inputmask1._mask = new Cleave(inputmask1, config1);

  // Delimiters Formatting
  const config2 = {
    delimiters: [".", "_", "-"],
    blocks: [3, 2, 3, 3],
    uppercase: true,
  };

  const inputmask2 = document.querySelector("#inputmask2");
  inputmask2._mask = new Cleave(inputmask2, config2);

  // Credit Card
  const config3 = {
    creditCard: true,
  };

  const inputmask3 = document.querySelector("#inputmask3");
  inputmask3._mask = new Cleave(inputmask3, config3);

  // Date Formatting
  const config4 = {
    date: true,
    delimiter: "-",
    datePattern: ["m", "d", "Y"],
  };

  const inputmask4 = document.querySelector("#inputmask4");
  inputmask4._mask = new Cleave(inputmask4, config4);

  // Time Formatting
  const config5 = {
    time: true,
    timePattern: ["h", "m", "s"],
  };

  const inputmask5 = document.querySelector("#inputmask5");
  inputmask5._mask = new Cleave(inputmask5, config5);

  // Phone Formatting
  const config6 = {
    phone: true,
    phoneRegionCode: "US",
  };

  const inputmask6 = document.querySelector("#inputmask6");
  inputmask6._mask = new Cleave(inputmask6, config6);

  // Numeral Formatting
  const config7 = {
    numeral: true,
    numeralThousandsGroupStyle: "thousand",
  };

  const inputmask7 = document.querySelector("#inputmask7");
  const maskUpdate = document.querySelector("#maskUpdate");

  maskUpdate.addEventListener("change", (evt) => {
    config7.numeralThousandsGroupStyle = evt.target.value;
    inputmask7._mask.destroy();
    inputmask7._mask = new Cleave(inputmask7, config7);
  });

  inputmask7._mask = new Cleave(inputmask7, config7);
};

window.addEventListener("app:mounted", onLoad, { once: true });
