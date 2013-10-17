/**
 * ClipGui
 */

package pt.adrz.clipx;

import java.awt.BorderLayout;
import java.awt.Color;
import java.awt.Container;
import java.awt.Dimension;
import java.awt.event.ActionEvent;
import java.awt.event.ActionListener;
import java.awt.event.KeyEvent;
import java.awt.event.KeyListener;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.event.MouseListener;

import javax.swing.JFrame;
import javax.swing.JMenu;
import javax.swing.JMenuBar;
import javax.swing.JMenuItem;
import javax.swing.JPanel;
import javax.swing.JPopupMenu;
import javax.swing.JScrollPane;
import javax.swing.JTextArea;
import javax.swing.ListSelectionModel;
import javax.swing.ScrollPaneConstants;
import javax.swing.SwingUtilities;
import javax.swing.event.ListSelectionEvent;
import javax.swing.event.ListSelectionListener;

public class ClipGUI extends JFrame implements ListSelectionListener, KeyListener, MouseListener, ClipboardListener, GuiState  {

	private static final long serialVersionUID = 4285795541593969626L;
	
	private static final String TITLE 					= "ClipX";
	private static final String RIGHT_CLICK_MENU_ITEM1 	= "activate";
	private static final String RIGHT_CLICK_MENU_ITEM2 	= "edit";
	private static final String RIGHT_CLICK_MENU_ITEM3 	= "delete";

	private Container 			container;
	
	private int 				xWindowDim = 600;
	private int 				yWindowDim = 400;
	private int 				visibleListRowCount = 10;
	
	private JPanel				panel1;
	private JPanel				panel2;
	private JTextArea		 	editTA;
	private JScrollPane			textAreaScrollPane;
	
	// menus
	private JMenuBar			menuBar;
	private JMenu				menuFile, menuEdit, menuAbout;
	private JMenuItem			menu1Item1, menu1Item2, menu1Item3, menuExitItem;
	
	private JPopupMenu			rightClickMenu;
	
	// List
	private ClipList			list;
	private JScrollPane			listScrollPane;
	
	private ClipSysTray 		clipSysTray;
	
	ClipManager 				clipManager;
	
	
	
	/**
	 * Constructor
	 */
	public ClipGUI() {
		
		super(TITLE);
		
		this.clipSysTray = new ClipSysTray(this);

		this.clipManager = new ClipManager();
		
		this.clipManager.setGuiState(this);

		this.clipManager.addClipboardListener(this);
		
		this.createMenu();
		
		this.createRightClickMenu();
		
		this.createList();

		this.createGUI();

		//this.tmp();
	}
	
	private void tmp() {

	}

	private void createMenu() {
		
		menuBar = new JMenuBar();
		
		menuFile = new JMenu("File");
		menuEdit = new JMenu("Edit");
		menuAbout = new JMenu("About");
		
		menu1Item1 = new JMenuItem("item1");
		menuExitItem = new JMenuItem("Exit");
		menuExitItem.addActionListener(new ActionListener() {
			@Override
			public void actionPerformed(ActionEvent e) {
				System.exit(0);
			}
		});

		menuFile.add(menu1Item1);
		menuFile.add(menuExitItem);
		
		menuBar.add(menuFile);
		menuBar.add(menuEdit);
		menuBar.add(menuAbout);
		this.setJMenuBar(menuBar);
	}
	
	private void createRightClickMenu() {

		this.rightClickMenu = new JPopupMenu();

		JMenuItem item1 = new JMenuItem(ClipGUI.RIGHT_CLICK_MENU_ITEM1);
		JMenuItem item2 = new JMenuItem(ClipGUI.RIGHT_CLICK_MENU_ITEM2);
		JMenuItem item3 = new JMenuItem(ClipGUI.RIGHT_CLICK_MENU_ITEM3);

		this.rightClickMenu.add(item1);
		this.rightClickMenu.add(item2);
		this.rightClickMenu.add(item3);
		
		// activate action
		item1.addMouseListener(new MouseAdapter() {

			public void mousePressed(MouseEvent e) { 

				//int index = list.locationToIndex(e.getPoint());
				int index = list.getSelectedIndex();
				
				if ( index == -1 ) return;
				
				// get the selected string from the filteredlist
				String selectedString = (String)list.getModel().getElementAt(index);
				
				// get the position from all the the items
				int pos = list.getModel().getItems().indexOf(selectedString);
				
				// set the clipboard
				clipManager.setClipboard(selectedString);
				
				list.getModel().switchVals(pos, selectedString);
				
				list.setSelectedIndex(0);
				list.getFilterField().setText("");
				editTA.setText(selectedString);
			}
		});

		// edit action
		item2.addMouseListener(new MouseAdapter() {

			public void mousePressed(MouseEvent e) { 

				if (editTA.isEditable())
					editTA.setEditable(false);
				else
					editTA.setEditable(true);
			}
		});

		// delete action
		item3.addMouseListener(new MouseAdapter() {

			public void mousePressed(MouseEvent e) { 

				try {
					
					int index = list.getSelectedIndex();
					list.getModel().remove(index);
					editTA.setText("");
					
					if (index == 0) { }
				}
				catch (IndexOutOfBoundsException eIndexOutBound) { }
			}
		});
	}
	
	private void createList() {
		
		list = new ClipList();

		list.getModel().addElement("first");
		list.getModel().addElement("secound");
		list.getModel().addElement("third");
		
		listScrollPane = new JScrollPane();

		list.setSelectionMode(ListSelectionModel.SINGLE_SELECTION);
		list.setSelectedIndex(0);
		list.addListSelectionListener(this);
		list.setVisibleRowCount(visibleListRowCount);
		listScrollPane = new JScrollPane(list,ScrollPaneConstants.VERTICAL_SCROLLBAR_ALWAYS,ScrollPaneConstants.HORIZONTAL_SCROLLBAR_NEVER);
		list.setPrototypeCellValue("tamanho"); // set horizontal size
		
		list.addMouseListener(new MouseAdapter() {
			
			public void mouseClicked(MouseEvent e) {
				
				if ( e.getClickCount() == 2 ) {
					
					int index = list.locationToIndex(e.getPoint());
					//int index = list.getSelectedIndex();
					
					// ... double click in an empty space
					if ( index == -1) {
						list.clearSelection();
						return;
					}
					
					// get the selected string from the filteredlist
					String selectedString = (String)list.getModel().getElementAt(index);
					
					// get the position from all the the items
					int pos = list.getModel().getItems().indexOf(selectedString);
					
					// set the clipboard
					clipManager.setClipboard(selectedString);
					
					list.getModel().switchVals(pos, selectedString);
					
					list.setSelectedIndex(0);
					list.getFilterField().setText("");
					editTA.setText(selectedString);
				}
			}
			
			public void mousePressed(MouseEvent e){
				
				if ( SwingUtilities.isRightMouseButton(e) ) 
					list.setSelectedIndex(list.locationToIndex(e.getPoint()));
			}
		});
		
		// listeners ...
		list.addMouseListener(this);
		list.addKeyListener(this);
	}

	private void createGUI() {
		
		container = this.getContentPane();

		container.setLayout(new BorderLayout());
		
		// panels ...
		panel1 = new JPanel();
		panel2 = new JPanel();
		panel1.setLayout(new BorderLayout());
		panel2.setLayout(new BorderLayout());	
		panel1.setBackground(Color.green);
		panel2.setBackground(Color.blue);
		
		// TextArea ...
		editTA = new JTextArea();
		editTA.setEditable(false);
		//getEditTA().getDocument().addDocumentListener(this);
		textAreaScrollPane 	= new JScrollPane(this.editTA,ScrollPaneConstants.VERTICAL_SCROLLBAR_ALWAYS,ScrollPaneConstants.HORIZONTAL_SCROLLBAR_ALWAYS);
		
		// add components ...
		container.add(panel1, BorderLayout.WEST);
		container.add(panel2, BorderLayout.CENTER);	
		panel1.add(list.getFilterField(), BorderLayout.NORTH);
		panel1.add(listScrollPane, BorderLayout.CENTER);
		panel2.add(textAreaScrollPane, BorderLayout.CENTER);
		
		this.setSize(xWindowDim, yWindowDim);
		this.setMinimumSize(new Dimension(xWindowDim, yWindowDim));
		this.setLocationRelativeTo(null);
		this.setDefaultCloseOperation(HIDE_ON_CLOSE);
		this.setVisible(true);
	}
	
	/**
	 * Detects changes in the list with clipboard items
	 */
	@Override
	public void valueChanged(ListSelectionEvent e) {	
		
		editTA.setEditable(false);
		
		// whenever the user makes a selection in the list, the text will be placed in the text area
		if (e.getValueIsAdjusting()) {
			return;
		}	
		else {
			editTA.setText((String)list.getModel().getElementAt(list.getSelectedIndex()));
		}
	}

	/**
	 * Event when some key is pressed. This listener is only added to the jlist component
	 * So far, only the delete key is implemented
	 */
	@Override
	public void keyPressed(KeyEvent arg0) {
		
		if (arg0.getKeyCode() == KeyEvent.VK_DELETE) {
			
			try {
				
				int index = list.getSelectedIndex();
				list.getModel().remove(index);
				editTA.setText("");
				
				if (index == 0) { }
			}
			catch (IndexOutOfBoundsException eIndexOutBound) {
				
			}
		}
	}

	@Override
	public void keyReleased(KeyEvent arg0) { }

	@Override
	public void keyTyped(KeyEvent arg0) { }

	@Override
	public void mouseClicked(MouseEvent arg0) { }

	@Override
	public void mouseEntered(MouseEvent arg0) { }

	@Override
	public void mouseExited(MouseEvent arg0) { }

	@Override
	public void mousePressed(MouseEvent e) {

		if ( SwingUtilities.isRightMouseButton(e) ) {

			list.setSelectedIndex(list.locationToIndex(e.getPoint()));
			this.rightClickMenu.show(e.getComponent(), e.getX(), e.getY());
		}
	}

	@Override
	public void mouseReleased(MouseEvent arg0) { }

	@Override
	public void newString(String copyString) {

		this.list.getModel().addElementTo(copyString, 0);
		this.editTA.setText(copyString);
	}

	@Override
	public ClipList getList() { return this.list; }
}
