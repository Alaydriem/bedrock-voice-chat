const onLoad = () => {
  // Card Number Input (Cleave js)
  const cardNumber = document.querySelector("#cardNumber");
  cardNumber._mask = new Cleave(cardNumber, {
    creditCard: true,
  });

  // Expiration date (Cleave js)
  const expireDate = document.querySelector("#expireDate");
  expireDate._mask = new Cleave(expireDate, {
    date: true,
    datePattern: ["m", "y"],
  });

  // CVV (Cleave js)
  const cvv = document.querySelector("#cvv");
  cvv._mask = new Cleave(cvv, { numeral: true });
};

window.addEventListener("app:mounted", onLoad, { once: true });
