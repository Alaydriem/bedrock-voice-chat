const onLoad = () => {
  const dropdownConfig = {
    placement: "bottom-start",
    modifiers: [
      {
        name: "offset",
        options: {
          offset: [0, 4],
        },
      },
    ],
  };

  new Popper(
    "#dropdown-wrapper1",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  new Popper(
    "#dropdown-wrapper2",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  new Popper(
    "#dropdown-wrapper3",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  new Popper(
    "#dropdown-wrapper4",
    ".popper-ref",
    ".popper-root",
    dropdownConfig
  );

  // Grid JS form HTML Table
  const gridTable1 = document.querySelector("#grid-table-1");
  const gridTableRef1 = gridTable1.querySelector("[data-grid-ref]");
  const gridTableWrapper1 = gridTable1.querySelector("[data-grid-wrapper]");

  gridTable1._table = new Gridjs.Grid({
    from: gridTableRef1,
    sort: true,
    search: true,
  }).render(gridTableWrapper1);

  // Grid JS from JSON data
  const products = [
    {
      id: 59,
      title: "Spring and summershoes",
      price: 20,
      quantity: 3,
      total: 60,
      discountPercentage: 8.71,
      discountedPrice: 55,
    },
    {
      id: 88,
      title: "TC Reusable Silicone Magic",
      price: 29,
      quantity: 2,
      total: 58,
      discountPercentage: 3.19,
      discountedPrice: 56,
    },
    {
      id: 18,
      title: "Oil Free Moisturizer 100ml",
      price: 40,
      quantity: 2,
      total: 80,
      discountPercentage: 13.1,
      discountedPrice: 70,
    },
    {
      id: 95,
      title: "Wholesale cargo lashing Belt",
      price: 930,
      quantity: 1,
      total: 930,
      discountPercentage: 17.67,
      discountedPrice: 766,
    },
    {
      id: 39,
      title: "Women Sweaters Wool",
      price: 600,
      quantity: 2,
      total: 1200,
      discountPercentage: 17.2,
      discountedPrice: 994,
    },
    {
      id: 96,
      title: "lighting ceiling kitchen",
      price: 30,
      quantity: 2,
      total: 60,
      discountPercentage: 14.89,
      discountedPrice: 51,
    },
    {
      id: 91,
      title: "Black Motorbike",
      price: 569,
      quantity: 3,
      total: 1707,
      discountPercentage: 13.63,
      discountedPrice: 1474,
    },
    {
      id: 9,
      title: "Infinix INBOOK",
      price: 1099,
      quantity: 1,
      total: 1099,
      discountPercentage: 11.83,
      discountedPrice: 969,
    },
    {
      id: 16,
      title: "Hyaluronic Acid Serum",
      price: 19,
      quantity: 1,
      total: 19,
      discountPercentage: 13.31,
      discountedPrice: 16,
    },
    {
      id: 54,
      title: "Pubg Printed Graphic T-Shirt",
      price: 46,
      quantity: 3,
      total: 138,
      discountPercentage: 16.44,
      discountedPrice: 115,
    },
  ];
  const gridTable2 = document.querySelector("#grid-table-2");

  gridTable2._table = new Gridjs.Grid({
    data: products,
    sort: true,
    search: true,
  }).render(gridTable2);

  // GridJS Async Table
  const gridConfig3 = {
    pagination: true,
    search: {
      server: {
        url: (prev, keyword) => `${prev}?search=${keyword}`,
      },
    },
    sort: true,
    columns: ["Title", "Director", "Producer"],
    server: {
      url: "https://swapi.py4e.com/api/films/",
      then: (data) =>
        data.results.map((movie) => [
          movie.title,
          movie.director,
          movie.producer,
        ]),
    },
  };
  const gridTable3 = document.querySelector("#grid-table-3");

  gridTable3._table = new Gridjs.Grid(gridConfig3).render(gridTable3);

  const gridTable4 = document.querySelector("#grid-table-4");

  const gridConfig4 = {
    columns: [
      {
        id: "id",
        name: "ID",
        formatter: (cell) => Gridjs.html(`<span class="mx-2">${cell}</span>`),
      },
      {
        id: "name",
        name: "Name",
        formatter: (cell) =>
          Gridjs.html(
            `<span class="text-slate-700 dark:text-navy-100 font-medium">${cell}</span>`
          ),
      },
      {
        id: "avatar_url",
        name: "Avatar",
        sort: false,
        formatter: (cell) =>
          Gridjs.html(`<div class="avatar flex">
                                  <img class="rounded-full" src="${cell}" alt="avatar">
                              </div>`),
      },
      {
        id: "email",
        name: "Email",
      },
      {
        id: "phone",
        name: "Phone Number",
      },
      {
        name: "Actions",
        sort: false,
        formatter: () =>
          Gridjs.html(`<div class="flex justify-center space-x-2">
                          <button onclick="$notification({ text: 'Item remove action', variant: 'warning' })" class="btn h-8 w-8 p-0 text-info hover:bg-info/20 focus:bg-info/20 active:bg-info/25">
                              <i class="fa fa-edit"></i>
                          </button>
                          <button onclick="$notification({ text: 'Item edit action', variant: 'info' })" class="btn h-8 w-8 p-0 text-error hover:bg-error/20 focus:bg-error/20 active:bg-error/25">
                              <i class="fa fa-trash-alt"></i>
                          </button>
                      </div>`),
      },
    ],
    data: [
      {
        id: "1",
        name: "John",
        email: "john@example.com",
        phone: "(01) 22 888 4444",
        avatar_url: "images/200x200.png",
      },
      {
        id: "2",
        name: "Doe",
        email: "thedoe@example.com",
        phone: "(33) 22 888 4444",
        avatar_url: "images/200x200.png",
      },
      {
        id: "3",
        name: "Nancy",
        email: "nancy@example.com",
        phone: "(21) 33 888 4444",
        avatar_url: "images/200x200.png",
      },
      {
        id: "4",
        name: "Clarke",
        email: "clarke@example.com",
        phone: "(44) 33 888 4444",
        avatar_url: "images/200x200.png",
      },
      {
        id: "5",
        name: "Robert",
        email: "robert@example.com",
        phone: "(27) 63 688 6444",
        avatar_url: "images/200x200.png",
      },
      {
        id: "6",
        name: "Tom",
        email: "thetom@example.com",
        phone: "(57) 63 688 6444",
        avatar_url: "images/200x200.png",
      },
      {
        id: "7",
        name: "Nolan",
        email: "Nolan@example.com",
        phone: "(27) 63 688 6444",
        avatar_url: "images/200x200.png",
      },
      {
        id: "8",
        name: "Adam",
        email: "Adam@example.com",
        phone: "(12) 22 888 4444",
        avatar_url: "images/200x200.png",
      },
      {
        id: "9",
        name: "Glen",
        email: "Glen@example.com",
        phone: "(74) 22 888 4444",
        avatar_url: "images/200x200.png",
      },
      {
        id: "10",
        name: "Edna",
        email: "Edna@example.com",
        phone: "(52) 33 888 4444",
        avatar_url: "images/200x200.png",
      },
      {
        id: "11",
        name: "Dianne",
        email: "dianne@example.com",
        phone: "(78) 33 888 4444",
        avatar_url: "images/200x200.png",
      },
      {
        id: "12",
        name: "Wilson",
        email: "wilson@example.com",
        phone: "(54) 63 688 6444",
        avatar_url: "images/200x200.png",
      },
      {
        id: "13",
        name: "Ross",
        email: "rose@example.com",
        phone: "(98) 63 688 6444",
        avatar_url: "images/200x200.png",
      },
      {
        id: "14",
        name: "Henry",
        email: "henry@example.com",
        phone: "(87) 63 688 6444",
        avatar_url: "images/200x200.png",
      },
      {
        id: "15",
        name: "Kerry",
        email: "kerry@example.com",
        phone: "(55) 63 688 6444",
        avatar_url: "images/200x200.png",
      },
    ],
    sort: true,
    search: true,
    pagination: {
      enabled: true,
      limit: 10,
    },
  };

  gridTable4._table = new Gridjs.Grid(gridConfig4).render(gridTable4);
};

window.addEventListener("app:mounted", onLoad, { once: true });
