export default () => ({
  init() {
    this.$watch("cardName", (val) => {
      switch (val) {
        case "mastercard":
          this.creditCardUI =
            "bg-gradient-to-r from-green-300 via-blue-500 to-purple-600";
          this.cardText = "Master Card";
          this.cardLogoSrc = "images/payments/cc-mastercard.svg";
          break;
        case "visa":
          this.creditCardUI = "bg-gradient-to-r from-indigo-400 to-indigo-600";
          this.cardLogoSrc = "images/payments/cc-visa.svg";
          this.cardText = "Visa";
          break;
        default:
          this.creditCardUI =
            "bg-gradient-to-r from-pink-300 via-purple-300 to-indigo-400";
          this.cardText = "Credit Card";
          this.cardLogoSrc = "";
      }
    });
  },
  cardNumber: null,
  cardText: "Credit Card",
  cardName: "",
  cardLogoSrc: "",
  nameOnCard: "",
  creditCardUI: "bg-gradient-to-r from-pink-300 via-purple-300 to-indigo-400",
  creditCardInput() {
    let am = this;
    return {
      creditCard: true,
      onCreditCardTypeChanged(type) {
        am.cardName = type;
      },
    };
  },
});
