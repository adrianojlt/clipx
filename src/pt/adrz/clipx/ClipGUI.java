/**
 * ClipGui
 */

package pt.adrz.clipx;

import java.awt.BorderLayout;
import java.awt.Color;
import java.awt.Container;
import java.awt.Dimension;
import java.awt.PopupMenu;
import java.awt.event.KeyEvent;
import java.awt.event.KeyListener;
import java.awt.event.MouseAdapter;
import java.awt.event.MouseEvent;
import java.awt.event.MouseListener;

import javax.swing.JButton;
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
import javax.swing.event.DocumentEvent;
import javax.swing.event.DocumentListener;
import javax.swing.event.ListSelectionEvent;
import javax.swing.event.ListSelectionListener;

public class ClipGUI extends JFrame implements ListSelectionListener, DocumentListener, KeyListener, MouseListener  {

	private static final long serialVersionUID = 4285795541593969626L;

	private Container 			container;
	
	private int 				xWindowDim = 600;
	private int 				yWindowDim = 400;
	private int 				visibleListRowCount = 10;
	
	private JPanel				panel1;
	private JPanel				panel2;
	final private JTextArea		editTA;
	private JScrollPane			textAreaScrollPane;
	
	// menus
	private JMenuBar			menuBar;
	private JMenu				menuFile, menuEdit, menuAbout;
	private JMenuItem			menu1Item1, menu1Item2, menu1Item3;
	
	private JPopupMenu			rightClickMenu;
	
	// List
	private ClipList			list;
	private JScrollPane			listScrollPane;
	
	// reference to ClipManager 
	ClipManager clipManager;
	
	
	
	/**
	 * Constructor
	 */
	public ClipGUI(final ClipManager clipManager) {
		
		super("ClipX");
		
		this.clipManager = clipManager;
		
		container = this.getContentPane();
		container.setLayout(new BorderLayout());
		
		this.createMenu();
		
		// right click menu
		this.rightClickMenu = new JPopupMenu();
		JMenuItem item1 = new JMenuItem("activate");
		JMenuItem item2 = new JMenuItem("delete");
		this.rightClickMenu.add(item1);
		this.rightClickMenu.add(item2);
		
		
		// List ...
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
		list.addMouseListener(this);
		
		list.addMouseListener(new MouseAdapter() {
			
			public void mouseClicked(MouseEvent e) {
				
				if ( e.getClickCount() == 2 ) {
					
					int index = list.locationToIndex(e.getPoint());
					
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
					getEditTA().setText(selectedString);
				}
			}
			
			public void mousePressed(MouseEvent e){
				
				if ( SwingUtilities.isRightMouseButton(e) ) {
					
					list.setSelectedIndex(list.locationToIndex(e.getPoint()));
					
					//JPopupMenu menu = new JPopupMenu();
					//JMenuItem item = new JMenuItem("click me ");
					//menu.add(item);
					//menu.show(e.getComponent(),e.getX(),e.getY());
					
					// create menu here ...
					//System.out.println("right click");
				}
			}
		});
		
		// listeners ...
		list.addKeyListener(this);
		
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
		getEditTA().getDocument().addDocumentListener(this);
		textAreaScrollPane 	= new JScrollPane(getEditTA(),ScrollPaneConstants.VERTICAL_SCROLLBAR_ALWAYS,ScrollPaneConstants.HORIZONTAL_SCROLLBAR_ALWAYS);
		
		// add components ...
		container.add(panel1, BorderLayout.WEST);
		container.add(panel2, BorderLayout.CENTER);	
		panel1.add(list.getFilterField(), BorderLayout.NORTH);
		panel1.add(listScrollPane, BorderLayout.CENTER);
		panel2.add(textAreaScrollPane, BorderLayout.CENTER);
		
		//container.add(new JButton("button"),BorderLayout.SOUTH);
		
		this.setSize(xWindowDim, yWindowDim);
		this.setMinimumSize(new Dimension(xWindowDim, yWindowDim));
		this.setLocationRelativeTo(null);
		this.setDefaultCloseOperation(HIDE_ON_CLOSE);
		this.setVisible(true);
	}

	private void createMenu() {
		
		menuBar = new JMenuBar();
		
		menuFile = new JMenu("File");
		menuEdit = new JMenu("Edit");
		menuAbout = new JMenu("About");
		
		menu1Item1 = new JMenuItem("item1");
		menuFile.add(menu1Item1);
		
		menuBar.add(menuFile);
		this.setJMenuBar(menuBar);
	}
	
	

	
	public ClipList getList() {
		return this.list;
	}
	

	/**
	 * Get Text Area
	 * @return the editTA
	 */
	public JTextArea getEditTA() {
		return editTA;
	}





	/**
	 * Detects changes in the list
	 */
	@Override
	public void valueChanged(ListSelectionEvent e) {	
		
		// whenever the user makes a selection in the list, the text will be placed in the text area
		if (e.getValueIsAdjusting()) {
			return;
		}	
		else {
			getEditTA().setText((String)list.getModel().getElementAt(list.getSelectedIndex()));
		}
	}





	@Override
	public void changedUpdate(DocumentEvent e) {
	}


	@Override
	public void insertUpdate(DocumentEvent e) {
	}



	@Override
	public void removeUpdate(DocumentEvent e) {
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
	public void mouseEntered(MouseEvent arg0) {
	}

	@Override
	public void mouseExited(MouseEvent arg0) {
	}

	@Override
	public void mousePressed(MouseEvent e) {
		if ( SwingUtilities.isRightMouseButton(e) ) {
			list.setSelectedIndex(list.locationToIndex(e.getPoint()));
			
			this.rightClickMenu.show(e.getComponent(), e.getX(), e.getY());
		}
	}

	@Override
	public void mouseReleased(MouseEvent arg0) {
	}

}
